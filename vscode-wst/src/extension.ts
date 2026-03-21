import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as os from 'os';
import * as path from 'path';
import * as fs from 'fs';

let diagnosticCollection: vscode.DiagnosticCollection;
const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();

export function activate(context: vscode.ExtensionContext) {
    diagnosticCollection = vscode.languages.createDiagnosticCollection('wst');
    context.subscriptions.push(diagnosticCollection);

    // Validate already-open wst documents
    vscode.workspace.textDocuments.forEach(doc => {
        if (doc.languageId === 'wst') validateDocument(doc);
    });

    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(doc => {
            if (doc.languageId === 'wst') validateDocument(doc);
        }),
        vscode.workspace.onDidChangeTextDocument(event => {
            if (event.document.languageId === 'wst') scheduleValidation(event.document);
        }),
        vscode.workspace.onDidSaveTextDocument(doc => {
            if (doc.languageId === 'wst') validateDocument(doc);
        }),
        vscode.workspace.onDidCloseTextDocument(doc => {
            diagnosticCollection.delete(doc.uri);
            cancelScheduled(doc.uri.toString());
        })
    );
}

function scheduleValidation(doc: vscode.TextDocument) {
    const key = doc.uri.toString();
    cancelScheduled(key);
    debounceTimers.set(key, setTimeout(() => {
        debounceTimers.delete(key);
        validateDocument(doc);
    }, 400));
}

function cancelScheduled(key: string) {
    const t = debounceTimers.get(key);
    if (t) { clearTimeout(t); debounceTimers.delete(key); }
}

function getWstExecutable(): string {
    const configured = vscode.workspace.getConfiguration('wst').get<string>('executablePath');
    if (configured) return configured;

    // Auto-detect: prefer workspace's own debug build
    for (const folder of vscode.workspace.workspaceFolders ?? []) {
        const candidate = path.join(folder.uri.fsPath, 'target', 'debug', 'wst');
        if (fs.existsSync(candidate)) return candidate;
    }

    return 'wst'; // fall back to PATH
}

function validateDocument(doc: vscode.TextDocument) {
    const wstExec = getWstExecutable();
    const isUnsaved = doc.isDirty || doc.uri.scheme !== 'file';

    let inputPath: string;
    let tempFile: string | null = null;

    if (isUnsaved) {
        tempFile = path.join(os.tmpdir(), `wst-validate-${Date.now()}-${Math.random().toString(36).slice(2)}.wst`);
        try {
            fs.writeFileSync(tempFile, doc.getText(), 'utf8');
        } catch {
            return;
        }
        inputPath = tempFile;
    } else {
        inputPath = doc.uri.fsPath;
    }

    const proc = cp.spawn(wstExec, ['-i', inputPath], { timeout: 5000 });
    let stderr = '';

    proc.stderr.on('data', chunk => { stderr += chunk.toString(); });

    proc.on('close', code => {
        cleanup(tempFile);

        if (code === 0) {
            diagnosticCollection.set(doc.uri, []);
            return;
        }

        const message = stderr.trim().replace(/^Error:\s*/, '') || 'Unknown error';
        const diag = new vscode.Diagnostic(
            new vscode.Range(0, 0, 0, Number.MAX_SAFE_INTEGER),
            message,
            vscode.DiagnosticSeverity.Error
        );
        diag.source = 'wst';
        diagnosticCollection.set(doc.uri, [diag]);
    });

    proc.on('error', () => cleanup(tempFile));
}

function cleanup(tempFile: string | null) {
    if (tempFile) {
        try { fs.unlinkSync(tempFile); } catch { /* ignore */ }
    }
}

export function deactivate() {
    diagnosticCollection?.dispose();
    debounceTimers.forEach(t => clearTimeout(t));
    debounceTimers.clear();
}

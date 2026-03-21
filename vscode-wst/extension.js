// @ts-check
'use strict';

const vscode = require('vscode');
const cp = require('child_process');
const os = require('os');
const path = require('path');
const fs = require('fs');

/** @type {vscode.DiagnosticCollection} */
let diagnosticCollection;
/** @type {Map<string, ReturnType<typeof setTimeout>>} */
const debounceTimers = new Map();
let _applyingBracketEdit = false;

/** @param {vscode.ExtensionContext} context */
function activate(context) {
    diagnosticCollection = vscode.languages.createDiagnosticCollection('wst');
    context.subscriptions.push(diagnosticCollection);

    vscode.workspace.textDocuments.forEach(doc => {
        if (doc.languageId === 'wst') validateDocument(doc);
    });

    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(doc => {
            if (doc.languageId === 'wst') validateDocument(doc);
        }),
        vscode.workspace.onDidChangeTextDocument(async event => {
            if (event.document.languageId !== 'wst') return;
            if (!_applyingBracketEdit) {
                for (const change of event.contentChanges) {
                    if (/^\n\s*$/.test(change.text)) {
                        await maybeConvertToBracketTags(event.document, change);
                    }
                }
            }
            scheduleValidation(event.document);
        }),
        vscode.workspace.onDidSaveTextDocument(doc => {
            if (doc.languageId === 'wst') validateDocument(doc);
        }),
        vscode.workspace.onDidCloseTextDocument(doc => {
            diagnosticCollection.delete(doc.uri);
            cancelScheduled(doc.uri.toString());
        }),
        vscode.languages.registerInlayHintsProvider(
            { language: 'wst' },
            { provideInlayHints }
        )
    );
}

// ── Auto bracket conversion ──────────────────────────────────────────────────

/**
 * When the user presses Enter while inside inline tags on a field,
 * automatically convert to square-bracket multi-line tag syntax.
 * @param {vscode.TextDocument} doc
 * @param {vscode.TextDocumentContentChangeEvent} change
 */
async function maybeConvertToBracketTags(doc, change) {
    const lineNo = change.range.start.line;
    if (lineNo + 1 >= doc.lineCount) return;

    const line = doc.lineAt(lineNo).text;
    const nextLine = doc.lineAt(lineNo + 1).text;
    const nextTrimmed = nextLine.trimStart();

    // Must look like a struct/variant field (has `identifier = type`)
    if (!line.match(/^\s*\w+\s*=\s*\S/)) return;
    // Not already bracket syntax
    if (line.includes('[') || line.includes(']')) return;
    // Line must not already be terminated
    if (/[,;{}]/.test(line.trimEnd().slice(-1))) return;

    const lineHasTags = /#\w/.test(line);
    const nextHasTags = nextTrimmed.startsWith('#');
    // Only trigger when tags are involved on either side
    if (!lineHasTags && !nextHasTags) return;

    const editor = vscode.window.visibleTextEditors.find(
        e => e.document.uri.toString() === doc.uri.toString()
    );
    if (!editor) return;

    const indent = /** @type {RegExpMatchArray} */ (line.match(/^(\s*)/)) [1];
    const innerIndent = indent + '    ';

    // Collect all tags from both lines
    const tagRe = /#[\w:]+(?:="[^"]*"|=[^\s,}[\]]+)?/g;
    const lineTags = line.match(tagRe) ?? [];
    const nextTags = nextTrimmed.match(tagRe) ?? [];
    const allTags = [...lineTags, ...nextTags];

    // Strip tags from line N to get the field+type part
    const fieldTypePart = line.replace(tagRe, '').trimEnd();

    // Strip tags from next line, then extract the terminator from what remains
    const nextStripped = nextTrimmed.replace(tagRe, '').trim();
    const terminator = nextStripped.match(/^[,}]/)?.[0] ?? '';

    const tagLines = allTags.length > 0
        ? allTags.map(t => `${innerIndent}${t}`).join('\n') + '\n'
        : `${innerIndent}\n`;

    const newText = `${fieldTypePart} [\n${tagLines}${indent}]${terminator}`;

    const replaceRange = new vscode.Range(lineNo, 0, lineNo + 1, nextLine.length);
    _applyingBracketEdit = true;
    try {
        // undoStopBefore: false merges this edit with the Enter keystroke so
        // a single Ctrl+Z reverts both in one step.
        await editor.edit(
            editBuilder => editBuilder.replace(replaceRange, newText),
            { undoStopBefore: false, undoStopAfter: false }
        );

        // If no tags existed yet, position cursor on the blank tag line inside brackets
        if (allTags.length === 0) {
            const pos = new vscode.Position(lineNo + 1, innerIndent.length);
            editor.selection = new vscode.Selection(pos, pos);
        }
    } finally {
        _applyingBracketEdit = false;
    }
}

// ── Inlay hints ──────────────────────────────────────────────────────────────

/**
 * @param {vscode.TextDocument} doc
 * @param {vscode.Range} range
 * @returns {vscode.InlayHint[]}
 */
function provideInlayHints(doc, range) {
    const text = doc.getText();
    const lines = text.split('\n');
    const hints = [];

    const defaultRequired = /^!default_level\s*=\s*required\s*;/m.test(text);
    const ghostLabel = defaultRequired ? '#required' : '#optional';
    const ghostTooltip = defaultRequired
        ? 'Implicitly required (default_level=required). Add #optional to override.'
        : 'Implicitly optional. Add #required to make it required.';

    let blockType = /** @type {string|null} */ (null);
    let braceDepth = 0;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trim();

        if (!trimmed || trimmed.startsWith('//')) continue;

        // Detect block keyword at top level
        if (braceDepth === 0) {
            const kw = trimmed.match(/^(?:private\s+)?(struct|enum|variant|protocol)\b/);
            if (kw) blockType = kw[1];
        }

        for (const ch of line) {
            if (ch === '{') braceDepth++;
            else if (ch === '}') {
                braceDepth--;
                if (braceDepth === 0) blockType = null;
            }
        }

        // Only hint inside struct blocks at depth 1
        if (blockType !== 'struct' || braceDepth !== 1) continue;
        if (!range.contains(new vscode.Position(i, 0))) continue;
        if (!trimmed.match(/^\w+\s*=/)) continue;

        // Collect full field text across bracket tags
        let fieldText = line;
        if (trimmed.includes('[')) {
            for (let j = i + 1; j < lines.length; j++) {
                fieldText += lines[j];
                if (lines[j].includes(']')) break;
            }
        }

        // Skip if field already has an explicit tag about optionality/requirement
        if (/#(?:required|optional|banned)\b/.test(fieldText)) continue;

        // Place hint after the type reference
        const hintCol =
            (line.match(/^(\s*\w+\s*=\s*[\w<>,. ]+?)(?:\s*[#,[\]{};]|$)/) ?? [])[1]?.trimEnd().length
            ?? line.trimEnd().length;

        const hint = new vscode.InlayHint(new vscode.Position(i, hintCol), ` ${ghostLabel}`);
        hint.tooltip = ghostTooltip;
        hint.paddingLeft = false;
        hints.push(hint);
    }

    return hints;
}

// ── Validation ───────────────────────────────────────────────────────────────

/** @param {vscode.TextDocument} doc */
function scheduleValidation(doc) {
    const key = doc.uri.toString();
    cancelScheduled(key);
    debounceTimers.set(key, setTimeout(() => {
        debounceTimers.delete(key);
        validateDocument(doc);
    }, 400));
}

/** @param {string} key */
function cancelScheduled(key) {
    const t = debounceTimers.get(key);
    if (t) { clearTimeout(t); debounceTimers.delete(key); }
}

function getWstExecutable() {
    const configured = vscode.workspace.getConfiguration('wst').get('executablePath');
    if (configured) return configured;

    const cargobin = path.join(os.homedir(), '.cargo', 'bin', 'wst');
    if (fs.existsSync(cargobin)) return cargobin;

    return 'wst';
}

/** @param {vscode.TextDocument} doc */
function validateDocument(doc) {
    const wstExec = getWstExecutable();
    const isUnsaved = doc.isDirty || doc.uri.scheme !== 'file';

    let inputPath;
    let tempFile = null;

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
        if (tempFile) { try { fs.unlinkSync(tempFile); } catch { } }

        if (code === 0) {
            diagnosticCollection.set(doc.uri, []);
            return;
        }

        const diagnostics = stderr.trim().split('\n')
            .map(line => line.trim().replace(/^Error:\s*/, ''))
            .filter(line => line.length > 0)
            .map(raw => {
                const posMatch = raw.match(/^(\d+):(\d+):\s*([\s\S]*)/);
                let range, message;
                if (posMatch) {
                    const line = parseInt(posMatch[1], 10) - 1;
                    const col = parseInt(posMatch[2], 10);
                    range = new vscode.Range(line, col, line, Number.MAX_SAFE_INTEGER);
                    message = posMatch[3];
                } else {
                    range = new vscode.Range(0, 0, 0, Number.MAX_SAFE_INTEGER);
                    message = raw;
                }
                const diag = new vscode.Diagnostic(range, message, vscode.DiagnosticSeverity.Error);
                diag.source = 'wst';
                return diag;
            });

        diagnosticCollection.set(doc.uri, diagnostics.length > 0 ? diagnostics : [
            Object.assign(new vscode.Diagnostic(
                new vscode.Range(0, 0, 0, Number.MAX_SAFE_INTEGER),
                'Unknown error', vscode.DiagnosticSeverity.Error
            ), { source: 'wst' })
        ]);
    });

    proc.on('error', () => {
        if (tempFile) { try { fs.unlinkSync(tempFile); } catch { } }
    });
}

function deactivate() {
    diagnosticCollection?.dispose();
    debounceTimers.forEach(t => clearTimeout(t));
    debounceTimers.clear();
}

module.exports = { activate, deactivate };

/**
 * CodeMirror 6 setup for Rhai scripts.
 *
 * This file initializes CodeMirror with syntax highlighting for Rhai,
 * the scripting language used by Blackwing.
 */

import { EditorView, basicSetup } from 'codemirror';
import { EditorState } from '@codemirror/state';
import { StreamLanguage } from '@codemirror/language';
import { oneDark } from '@codemirror/theme-one-dark';

// Custom language mode for Rhai scripts
const rhaiLanguage = StreamLanguage.define({
    name: 'rhai',

    startState() {
        return {
            inString: false,
            stringChar: null,
            inMultilineComment: false,
        };
    },

    token(stream, state) {
        // Handle multiline comments
        if (state.inMultilineComment) {
            if (stream.match(/\*\//)) {
                state.inMultilineComment = false;
                return 'comment';
            }
            stream.next();
            return 'comment';
        }

        // Start of multiline comment
        if (stream.match(/\/\*/)) {
            state.inMultilineComment = true;
            return 'comment';
        }

        // Single-line comments
        if (stream.match(/\/\/.*/)) {
            return 'comment';
        }

        // Strings
        if (stream.match(/"([^"\\]|\\.)*"/)) {
            return 'string';
        }
        if (stream.match(/'([^'\\]|\\.)*'/)) {
            return 'string';
        }

        // Template strings
        if (stream.match(/`([^`\\]|\\.)*`/)) {
            return 'string';
        }

        // Numbers (including hex, octal, binary)
        if (stream.match(/0x[0-9a-fA-F_]+/)) {
            return 'number';
        }
        if (stream.match(/0o[0-7_]+/)) {
            return 'number';
        }
        if (stream.match(/0b[01_]+/)) {
            return 'number';
        }
        if (stream.match(/\d[\d_]*\.[\d_]*([eE][+-]?\d+)?/)) {
            return 'number';
        }
        if (stream.match(/\d[\d_]*/)) {
            return 'number';
        }

        // Keywords
        if (stream.match(/\b(let|const|if|else|while|loop|for|in|break|continue|return|throw|try|catch|fn|private|import|export|as|switch|case|default|true|false|nil|this|global|print|debug|type_of|is_def_var|is_def_fn)\b/)) {
            return 'keyword';
        }

        // Built-in types
        if (stream.match(/\b(bool|int|float|char|string|array|map|blob|timestamp|range)\b/)) {
            return 'type';
        }

        // Operators
        if (stream.match(/[+\-*/%&|^!<>=?:]+/)) {
            return 'operator';
        }

        // Function calls
        if (stream.match(/[a-zA-Z_][a-zA-Z0-9_]*(?=\s*\()/)) {
            return 'function';
        }

        // Identifiers
        if (stream.match(/[a-zA-Z_][a-zA-Z0-9_]*/)) {
            return 'variableName';
        }

        // Punctuation
        if (stream.match(/[{}()\[\];,\.]/)) {
            return 'punctuation';
        }

        // Skip other characters
        stream.next();
        return null;
    },
});

// Store editor instances by element
const editors = new WeakMap();

// Track if we're updating from Rust (to avoid feedback loops)
let updatingFromRust = false;

/**
 * Initialize CodeMirror on an element.
 * @param {HTMLElement} element - The container element
 * @param {string} content - Initial content
 * @param {function} callback - Called when content changes
 */
window.initCodeMirror = function(element, content, callback) {
    // Check if already initialized
    if (editors.has(element)) {
        return;
    }

    const state = EditorState.create({
        doc: content,
        extensions: [
            basicSetup,
            oneDark,
            rhaiLanguage,
            EditorView.updateListener.of((update) => {
                if (update.docChanged && !updatingFromRust) {
                    callback(update.state.doc.toString());
                }
            }),
            // Custom theme overrides for dark mode
            EditorView.theme({
                '&': {
                    backgroundColor: 'rgb(15 23 42)', // slate-900
                    color: '#e2e8f0', // slate-200
                },
                '.cm-gutters': {
                    backgroundColor: 'rgb(30 41 59)', // slate-800
                    borderRight: '1px solid rgb(51 65 85)', // slate-700
                },
                '.cm-activeLineGutter': {
                    backgroundColor: 'rgb(51 65 85)', // slate-700
                },
            }),
        ],
    });

    const view = new EditorView({
        state,
        parent: element,
    });

    editors.set(element, view);
};

/**
 * Update the content of a CodeMirror instance.
 * @param {HTMLElement} element - The container element
 * @param {string} content - New content
 */
window.updateCodeMirrorContent = function(element, content) {
    const view = editors.get(element);
    if (!view) return;

    // Check if content actually changed
    if (view.state.doc.toString() === content) {
        return;
    }

    updatingFromRust = true;
    view.dispatch({
        changes: {
            from: 0,
            to: view.state.doc.length,
            insert: content,
        },
    });
    updatingFromRust = false;
};

/**
 * Destroy a CodeMirror instance.
 * @param {HTMLElement} element - The container element
 */
window.destroyCodeMirror = function(element) {
    const view = editors.get(element);
    if (view) {
        view.destroy();
        editors.delete(element);
    }
};

/**
 * Get the current content of a CodeMirror instance.
 * @param {HTMLElement} element - The container element
 * @returns {string|null} The content, or null if not initialized
 */
window.getCodeMirrorContent = function(element) {
    const view = editors.get(element);
    return view ? view.state.doc.toString() : null;
};

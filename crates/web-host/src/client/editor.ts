// Copyright (C) 2025 Ryan Daum <ryan.daum@gmail.com> This program is free
// software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, version
// 3.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.
//

import * as monaco from "monaco-editor";

// TODO: MOO syntax highlighting format validation
//   what exists in editor.js is just a first crack at this, it's by no means complete.
const MooSyntax = {
    // Set defaultToken to invalid to see what you do not tokenize yet
    defaultToken: "invalid",
    tokenPostfix: ".moo",
    ignoreCase: true,

    keywords: [
        "if",
        "else",
        "elseif",
        "endif",
        "while",
        "endwhile",
        "for",
        "endfor",
        "fork",
        "endfork",
        "try",
        "endtry",
        "catch",
        "except",
        "let",
        "const",
        "global",
    ],

    typeKeywords: [
        "this",
    ],

    operators: [
        "<=",
        ">=",
        "==",
        "!=",
        "+",
        "-",
        "*",
        "/",
        "%",
        "|",
        "^",
        "!",
        "~",
        "&&",
        "||",
        "?",
        ":",
        "=",
        "@",
        "$",
    ],

    errors: [
        "E_TYPE",
        "E_DIV",
        "E_PERM",
        "E_PROPNF",
        "E_VERBNF",
        "E_VARNF",
        "E_INVIND",
        "E_RECMOVE",
        "E_MAXREC",
        "E_RANGE",
        "E_ARGS",
        "E_NACC",
        "E_INVARG",
        "E_QUOTA",
        "E_FLOAT",
        "E_NONE",
    ],

    // we include these common regular expressions
    symbols: /[=><!~?:&|+\-*\/\^%]+/,
    escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,
    digits: /\d+(_+\d+)*/,
    octaldigits: /[0-7]+(_+[0-7]+)*/,
    binarydigits: /[0-1]+(_+[0-1]+)*/,
    hexdigits: /[[0-9a-fA-F]+(_+[0-9a-fA-F]+)*/,

    // References $fdsfds_fdsaf $fcdsaf etc
    sysref: /\$([a-zA-Z_][a-zA-Z0-9_]*)/,
    // Object references
    object_ref: /#(\d+)/,

    // The main tokenizer
    tokenizer: {
        root: [
            [/[{}]/, "delimiter.bracket"],
            { include: "common" },
        ],

        common: [
            [/@object_ref/, "type.namespace"],
            [/@sysref/, "type.namespace"],

            // identifiers and keywords
            [/[a-z_$][\w$]*/, {
                cases: {
                    "@errors": "constant",
                    "@typeKeywords": "keyword",
                    "@keywords": "keyword",
                    "@default": "identifier",
                },
            }],

            // whitespace
            { include: "@whitespace" },

            // delimiters and operators
            [/[()\[\]]/, "@brackets"],
            [/[<>](?!@symbols)/, "@brackets"],
            [/@symbols/, {
                cases: {
                    "@operators": "delimiter",
                    "@default": "",
                },
            }],

            // numbers
            [/(@digits)[eE]([\-+]?(@digits))?/, "number.float"],
            [/(@digits)\.(@digits)([eE][\-+]?(@digits))?/, "number.float"],
            [/0[xX](@hexdigits)/, "number.hex"],
            [/0[oO]?(@octaldigits)/, "number.octal"],
            [/0[bB](@binarydigits)/, "number.binary"],
            [/(@digits)/, "number"],

            // delimiter: after number because of .\d floats
            [/[;,.]/, "delimiter"],

            // strings
            [/"([^"\\]|\\.)*$/, "string.invalid"], // non-terminated string
            [/"/, "string", "@string_double"],
        ],

        whitespace: [
            [/[ \t\r\n]+/, ""],
        ],

        string_double: [
            [/[^\\"]+/, "string"],
            [/@escapes/, "string.escape"],
            [/\\./, "string.escape.invalid"],
            [/"/, "string", "@pop"],
        ],

        bracketCounting: [
            [/\{/, "delimiter.bracket", "@bracketCounting"],
            [/\}/, "delimiter.bracket", "@pop"],
            { include: "common" },
        ],
    },
};

// TODO: completion provider which works to lookup verbs / props for constant $references and #object_ids

let model_counter = 0;

export function createEditor(editor_element) {
    editor_element.innerHTML = "";
    monaco.languages.setMonarchTokensProvider("moo", MooSyntax);
    // Each model gets a unique URI, by incrementing a counter.
    let uri = monaco.Uri.parse("file:///model" + model_counter++);

    let model = monaco.editor.createModel("", "moo", uri);
    let editor = monaco.editor.create(editor_element, {
        value: "",
        language: "moo",
        theme: "vs-dark",
        readOnly: false,
        roundedSelection: true,
        scrollBeyondLastLine: false,
        fixedOverflowWidgets: true,
        automaticLayout: true,
        minimap: {
            enabled: true,
        },
        fontSize: "12px",
        fontFamily: "monospace",
        colorDecorators: true,
        dragAndDrop: false,
        emptySelectionClipboard: false,
        autoClosingDelete: "never",
    });
    editor.setModel(model);
    return model;
}

export function updateEditor(editor, source) {
    editor.setValue(source.join("\n"));
    editor.setLanguage("moo");
}

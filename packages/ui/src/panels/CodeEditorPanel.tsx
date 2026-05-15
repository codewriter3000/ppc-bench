import { type Component, For, Show, createMemo, createSignal } from "solid-js";
import type { AssembleError } from "@ppc-bench/kernel";
import { Panel } from "../shell/Panel";
import "../styles/code-editor.css";

export interface CodeEditorPanelProps {
  source: string;
  onSourceChange: (next: string) => void;
  errors?: readonly AssembleError[];
  readOnly?: boolean;
}

// ── Tokenizer ────────────────────────────────────────────────────────────
const KEYWORDS = new Set([
  "li","lis","mr","nop","blr","bctr","blrl","trap","sync","isync","sc",
  "add","addi","addis","subf","neg","mullw","divw","and","or","xor",
  "nand","nor","xori","xoris","andi","andis","ori","oris",
  "cmp","cmpi","cmpl","cmpli","cmpw","cmpwi","cmplw","cmplwi",
  "lwz","lwzu","lwzx","lwzux","lhz","lhzu","lha","lhau","lbz","lbzu",
  "stw","stwu","stwx","stwux","sth","sthu","stb","stbu","lmw","stmw",
  "b","ba","bl","bla","beq","bne","blt","bgt","ble","bge","bso","bns",
  "rlwinm","rlwnm","rlwimi","slw","srw","sraw","srawi",
  "mfspr","mtspr","mflr","mtlr","mfctr","mtctr","mfcr","mtcrf",
  "crand","cror","crxor","crnand","crnor","creqv","crandc","crorc",
  "fadd","fsub","fmul","fdiv","fmadd","fmsub","fnmadd","fnmsub",
  "fadds","fsubs","fmuls","fdivs","fmadds","fmsubs","fnmadds","fnmsubs",
  "fabs","fneg","fmr","fsel","fres","frsqrte","frsp","fcmpu","fcmpo",
  "lfs","lfd","stfs","stfd",
  "ps_add","ps_sub","ps_mul","ps_div","ps_madd","ps_msub",
  "ps_mr","ps_neg","ps_abs","ps_merge00","ps_merge01","ps_merge10","ps_merge11",
  "psq_l","psq_st","psq_lu","psq_stu","psq_lx","psq_stx",
  // Section directives
  ".data",".text",".rodata",".bss",".section",
  // Data directives
  ".byte",".short",".hword",".2byte",
  ".long",".word",".int",".4byte",
  ".float",".single",".double",
  ".string",".asciz",".ascii",
  ".zero",".space",".align",".balign",
]);

type Token = { kind: string; text: string };

const tokenizeLine = (line: string): Token[] => {
  const tokens: Token[] = [];
  let i = 0;
  let sawMnemonic = false;
  while (i < line.length) {
    const c = line.charCodeAt(i);
    if (line[i] === "#" || line[i] === ";") {
      tokens.push({ kind: "comment", text: line.slice(i) });
      return tokens;
    }
    // Quoted string literal — consume including any '#' inside.
    if (line[i] === '"') {
      let j = i + 1;
      while (j < line.length && line[j] !== '"') {
        if (line[j] === '\\') j++; // skip escaped character
        j++;
      }
      if (j < line.length) j++; // include closing quote
      tokens.push({ kind: "string", text: line.slice(i, j) });
      i = j; continue;
    }
    if (c === 0x20 || c === 0x09) {
      let j = i;
      while (j < line.length && (line.charCodeAt(j) === 0x20 || line.charCodeAt(j) === 0x09)) j++;
      tokens.push({ kind: "ws", text: line.slice(i, j) });
      i = j; continue;
    }
    if (line[i] === "," || line[i] === "(" || line[i] === ")") {
      tokens.push({ kind: "punct", text: line[i]! });
      i++; continue;
    }
    if (
      /[0-9]/.test(line[i]!) ||
      ((line[i] === "-" || line[i] === "+") && /[0-9]/.test(line[i + 1] ?? ""))
    ) {
      let j = i + 1;
      while (j < line.length && /[0-9a-fA-FxXbB_-]/.test(line[j]!)) j++;
      tokens.push({ kind: "num", text: line.slice(i, j) });
      i = j; continue;
    }
    if (/[A-Za-z_.]/.test(line[i]!)) {
      let j = i + 1;
      while (j < line.length && /[A-Za-z0-9_.]/.test(line[j]!)) j++;
      const text = line.slice(i, j);
      if (line[j] === ":") {
        tokens.push({ kind: "label", text });
        tokens.push({ kind: "punct", text: ":" });
        i = j + 1; continue;
      }
      if (/^(r|f)\d{1,2}$/i.test(text) || /^cr[0-7]$/i.test(text)) {
        tokens.push({ kind: "reg", text });
        i = j; continue;
      }
      if (!sawMnemonic) {
        sawMnemonic = true;
        const lower = text.toLowerCase();
        tokens.push({ kind: KEYWORDS.has(lower) || /^b[a-z]+$/.test(lower) ? "mnem" : "ident", text });
      } else {
        tokens.push({ kind: "ident", text });
      }
      i = j; continue;
    }
    tokens.push({ kind: "other", text: line[i]! });
    i++;
  }
  return tokens;
};

const CSS_COLOR: Record<string, string> = {
  comment: "var(--color-text-muted)",
  label:   "var(--color-success)",
  mnem:    "var(--color-primary)",
  reg:     "#a3007a",
  num:     "#a14b00",
  punct:   "var(--color-text-muted)",
  string:  "#22863a",
  ident:   "var(--color-text)",
  ws:      "var(--color-text)",
  other:   "var(--color-text)",
};
const CSS_EXTRA: Record<string, string> = {
  mnem:    "font-weight:600;",
  label:   "font-weight:600;",
  comment: "font-style:italic;",
};

const escHtml = (s: string) =>
  s.replace(/[&<>"]/g, (c) =>
    c === "&" ? "&amp;" : c === "<" ? "&lt;" : c === ">" ? "&gt;" : "&quot;",
  );

const highlight = (source: string): string => {
  const lines = source.split("\n");
  return lines
    .map((line) => {
      if (!line) return "";
      return tokenizeLine(line)
        .map((t) => {
          const col = CSS_COLOR[t.kind] ?? "var(--color-text)";
          const extra = CSS_EXTRA[t.kind] ?? "";
          return `<span style="color:${col};${extra}">${escHtml(t.text)}</span>`;
        })
        .join("");
    })
    .join("\n") + "\n";
};

export const CodeEditorPanel: Component<CodeEditorPanelProps> = (props) => {
  const lineCount = createMemo(() => Math.max(1, props.source.split("\n").length));
  const errorLines = createMemo(() => new Set((props.errors ?? []).map((e) => e.line)));
  const html = createMemo(() => highlight(props.source));
  const [scrollTop, setScrollTop] = createSignal(0);
  let preEl: HTMLPreElement | undefined;
  let gutterEl: HTMLDivElement | undefined;

  const onScroll = (e: Event) => {
    const ta = e.currentTarget as HTMLTextAreaElement;
    setScrollTop(ta.scrollTop);
    if (preEl) { preEl.scrollTop = ta.scrollTop; preEl.scrollLeft = ta.scrollLeft; }
    if (gutterEl) gutterEl.scrollTop = ta.scrollTop;
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const ta = e.currentTarget as HTMLTextAreaElement;
    const isTab = e.key === "Tab";
    if (!isTab) return;

    e.preventDefault();

    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const isShiftTab = e.shiftKey;
    const hasSelection = start !== end;

    // Get the full source
    const before = props.source.substring(0, start);
    const selected = props.source.substring(start, end);
    const after = props.source.substring(end);

    // Find the start of the line containing the cursor
    let lineStart = before.lastIndexOf("\n") + 1;

    // Determine if we have a selection spanning multiple lines
    const hasMultilineSelection = selected.includes("\n");

    let newSource: string;
    let newSelectionStart: number;
    let newSelectionEnd: number;
    const indent = "  "; // 2 spaces

    if (hasMultilineSelection || (start !== end && selected !== "")) {
      // Multi-line selection or non-empty selection
      const beforeSelection = props.source.substring(0, start);
      const linesBeforeStart = beforeSelection.split("\n");
      const selectedStartLine = linesBeforeStart.length - 1;
      const selectedEndLine = selectedStartLine + selected.split("\n").length - 1;

      const lines = props.source.split("\n");
      const processedLines = lines.map((line, i) => {
        const isInSelection = i >= selectedStartLine && i <= selectedEndLine;
        if (!isInSelection) return line;

        if (isShiftTab) {
          // Remove indentation
          if (line.startsWith(indent)) return line.slice(indent.length);
          return line;
        } else {
          // Add indentation
          return indent + line;
        }
      });

      newSource = processedLines.join("\n");

      // Calculate new selection positions
      if (isShiftTab) {
        // For unindent, we need to check how many lines actually had indentation removed
        let startAdjust = 0;
        let endAdjust = 0;

        // Count indent removals from start line
        for (let i = selectedStartLine; i <= selectedEndLine; i++) {
          const originalLine = lines[i]!;
          const newLine = processedLines[i]!;
          if (i === selectedStartLine) {
            startAdjust = originalLine.length - newLine.length;
          }
          if (i === selectedEndLine) {
            endAdjust = originalLine.length - newLine.length;
          }
        }

        newSelectionStart = Math.max(lineStart, start - startAdjust);
        newSelectionEnd = end - endAdjust;
      } else {
        // For indent, all selected lines get indented
        newSelectionStart = start + indent.length;
        newSelectionEnd = end + indent.length * (selectedEndLine - selectedStartLine + 1);
      }
    } else {
      // No selection or empty selection - indent/unindent current line
      const lineEnd = after.indexOf("\n");
      const fullLine = before.slice(lineStart) + after.slice(0, lineEnd !== -1 ? lineEnd : after.length);

      let newLineContent: string;
      let lineIndentAdjust = 0;

      if (isShiftTab) {
        // Remove indentation
        if (fullLine.startsWith(indent)) {
          newLineContent = fullLine.slice(indent.length);
          lineIndentAdjust = -indent.length;
        } else {
          newLineContent = fullLine;
          lineIndentAdjust = 0;
        }
      } else {
        // Add indentation
        newLineContent = indent + fullLine;
        lineIndentAdjust = indent.length;
      }

      const beforeLineStart = before.slice(0, lineStart);
      const afterLineEnd = after.slice(lineEnd);
      newSource = beforeLineStart + newLineContent + afterLineEnd;

      // For cursor-only (no selection), move cursor with indentation
      if (!hasSelection) {
        newSelectionStart = start + lineIndentAdjust;
        newSelectionEnd = start + lineIndentAdjust;
      } else {
        // For single-line selection, adjust both start and end
        newSelectionStart = start + lineIndentAdjust;
        newSelectionEnd = end + lineIndentAdjust;
      }
    }

    props.onSourceChange(newSource);

    // Restore selection after state updates
    setTimeout(() => {
      ta.selectionStart = newSelectionStart;
      ta.selectionEnd = newSelectionEnd;
      ta.focus();
    }, 0);
  };

  return (
    <Panel
      title="Source"
      grow
      actions={
        <span style="color:var(--color-text-muted);font-size:var(--size-caption);font-family:var(--font-mono);">
          {lineCount()} lines
        </span>
      }
      bodyStyle="flex:1 1 auto;display:flex;flex-direction:column;min-height:0;overflow:hidden;"
    >
      <div class="editor__wrap">
        <div ref={gutterEl} class="editor__gutter" aria-hidden="true">
          <div style={`transform:translateY(-${scrollTop()}px);`}>
            <For each={Array.from({ length: lineCount() }, (_, i) => i + 1)}>
              {(n) => (
                <div class={`editor__gutter-line${errorLines().has(n) ? " editor__gutter-line--error" : ""}`}>
                  {errorLines().has(n) ? `● ${n}` : n}
                </div>
              )}
            </For>
          </div>
        </div>
        <div class="editor__box">
          <pre ref={preEl} class="editor__highlight" aria-hidden="true" innerHTML={html()} />
          <textarea
            class="editor__textarea"
            spellcheck={false}
            readOnly={props.readOnly}
            value={props.source}
            onInput={(e) => props.onSourceChange(e.currentTarget.value)}
            onScroll={onScroll}
            onKeyDown={onKeyDown}
            placeholder="# Write PPC assembly here — try:&#10;li r3, 1&#10;li r4, 41&#10;add r5, r3, r4&#10;blr"
          />
        </div>
      </div>
      <Show when={props.errors && props.errors.length > 0}>
        <div class="editor__errors">
          <For each={props.errors}>
            {(err) => <div><strong>line {err.line}:</strong> {err.message}</div>}
          </For>
        </div>
      </Show>
    </Panel>
  );
};

import { useRef, useEffect } from "react";
import type { LogEntry } from "../types";

function lineClass(message: string, stream: string): string {
  if (stream === "err" || /error|failed|not found|could not|kick|banned/i.test(message)) return "err";
  if (/started|welcome|connected|joined|ready/i.test(message)) return "ok";
  if (/warning|warn/i.test(message)) return "warn";
  return "";
}

export function Terminal({
  lines,
  height = 300,
  follow = true,
}: {
  lines: LogEntry[];
  height?: number;
  follow?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (follow && ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
    }
  }, [lines, follow]);

  return (
    <div className="terminal" ref={ref} style={{ maxHeight: height }}>
      {lines.length === 0 && <div className="term-empty">Nessun output. Avvia il server per vedere i log.</div>}
      {lines.map((line, index) => (
        <div className="term-line" key={`${index}-${line.ts}`}>
          <span className="term-ts">{line.ts}</span>
          <span className={`term-msg ${lineClass(line.message, line.stream)}`}>{line.message}</span>
        </div>
      ))}
    </div>
  );
}

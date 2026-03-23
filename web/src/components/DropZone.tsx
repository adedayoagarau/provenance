"use client";

import { useCallback, useState, DragEvent } from "react";

interface DropZoneProps {
  onFiles: (files: File[]) => void;
  multiple?: boolean;
  disabled?: boolean;
}

const ACCEPTED = ".docx,.txt,.md,.pdf";
const ACCEPTED_TYPES = [
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "text/plain",
  "text/markdown",
  "application/pdf",
];

export default function DropZone({ onFiles, multiple = false, disabled = false }: DropZoneProps) {
  const [isDragging, setIsDragging] = useState(false);

  const validateFiles = (files: File[]): File[] => {
    return files.filter((f) => {
      const ext = "." + f.name.split(".").pop()?.toLowerCase();
      return [".docx", ".txt", ".md", ".pdf"].includes(ext) ||
        ACCEPTED_TYPES.includes(f.type);
    });
  };

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!disabled) setIsDragging(true);
  }, [disabled]);

  const handleDragLeave = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback(
    (e: DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragging(false);
      if (disabled) return;

      const files = Array.from(e.dataTransfer.files);
      const valid = validateFiles(files);
      if (valid.length > 0) {
        onFiles(multiple ? valid : [valid[0]]);
      }
    },
    [onFiles, multiple, disabled]
  );

  const handleClick = () => {
    if (disabled) return;
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ACCEPTED;
    input.multiple = multiple;
    input.onchange = () => {
      if (input.files) {
        const valid = validateFiles(Array.from(input.files));
        if (valid.length > 0) {
          onFiles(multiple ? valid : [valid[0]]);
        }
      }
    };
    input.click();
  };

  return (
    <div
      onClick={handleClick}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`
        border border-dashed transition-all cursor-pointer
        flex flex-col items-center justify-center gap-3 py-16 px-8
        ${disabled ? "opacity-40 cursor-not-allowed" : ""}
        ${isDragging
          ? "border-accent bg-status-blue-dim/30"
          : "border-border hover:border-border-focus"
        }
      `}
    >
      <div className="font-mono text-2xl text-text-tertiary">&#8593;</div>
      <div className="text-sm text-text-secondary">
        {multiple ? "Drop files here" : "Drop a file here"} or{" "}
        <span className="text-accent underline">browse</span>
      </div>
      <div className="text-xs text-text-tertiary font-mono">
        .docx &middot; .txt &middot; .md &middot; .pdf
      </div>
    </div>
  );
}

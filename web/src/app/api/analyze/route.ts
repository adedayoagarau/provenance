import { NextRequest, NextResponse } from "next/server";
import { writeFile, unlink, mkdir } from "fs/promises";
import { execFile } from "child_process";
import { promisify } from "util";
import { join } from "path";
import { randomUUID } from "crypto";

const execFileAsync = promisify(execFile);

const PROVENANCE_BIN =
  process.env.PROVENANCE_BIN || join(process.cwd(), "..", "target", "release", "provenance");
const UPLOAD_DIR = process.env.UPLOAD_DIR || "/tmp/provenance-uploads";
const ACCEPTED_EXTENSIONS = [".docx", ".txt", ".md", ".pdf"];
const MAX_FILE_SIZE = 100 * 1024 * 1024; // 100MB

export async function POST(request: NextRequest) {
  let tempPath: string | null = null;

  try {
    const formData = await request.formData();
    const file = formData.get("file") as File | null;

    if (!file) {
      return NextResponse.json({ error: "No file provided" }, { status: 400 });
    }

    if (file.size > MAX_FILE_SIZE) {
      return NextResponse.json(
        { error: "File exceeds 100MB limit" },
        { status: 400 }
      );
    }

    const ext = "." + file.name.split(".").pop()?.toLowerCase();
    if (!ACCEPTED_EXTENSIONS.includes(ext)) {
      return NextResponse.json(
        { error: `Unsupported file type: ${ext}. Accepted: ${ACCEPTED_EXTENSIONS.join(", ")}` },
        { status: 400 }
      );
    }

    // Save to temp directory
    await mkdir(UPLOAD_DIR, { recursive: true });
    const safeFileName = `${randomUUID()}${ext}`;
    tempPath = join(UPLOAD_DIR, safeFileName);
    const buffer = Buffer.from(await file.arrayBuffer());
    await writeFile(tempPath, buffer);

    // Run provenance analyze
    const { stdout, stderr } = await execFileAsync(PROVENANCE_BIN, [
      "analyze",
      "--file",
      tempPath,
      "--format",
      "json",
    ], {
      timeout: 120000,
      maxBuffer: 50 * 1024 * 1024,
      env: { ...process.env, RUST_LOG: "warn" },
    });

    // Parse JSON output
    let result;
    try {
      result = JSON.parse(stdout);
    } catch {
      return NextResponse.json(
        {
          error: "Failed to parse analysis output",
          details: stderr || stdout.slice(0, 500),
        },
        { status: 500 }
      );
    }

    return NextResponse.json({
      file_name: file.name,
      file_size: file.size,
      result,
    });
  } catch (err: unknown) {
    const error = err as Error & { code?: string; stderr?: string };
    console.error("Analysis error:", error);
    return NextResponse.json(
      {
        error: "Analysis failed",
        details: error.stderr || error.message,
      },
      { status: 500 }
    );
  } finally {
    // Clean up temp file
    if (tempPath) {
      unlink(tempPath).catch(() => {});
    }
  }
}

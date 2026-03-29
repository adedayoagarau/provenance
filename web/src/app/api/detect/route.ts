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
const MAX_TEXT_LENGTH = 50000; // 50K characters

export async function POST(request: NextRequest) {
  let tempPath: string | null = null;

  try {
    const body = await request.json();
    const text = body.text as string | undefined;

    if (!text || text.trim().length === 0) {
      return NextResponse.json({ error: "No text provided" }, { status: 400 });
    }

    if (text.length > MAX_TEXT_LENGTH) {
      return NextResponse.json(
        { error: `Text exceeds ${MAX_TEXT_LENGTH} character limit` },
        { status: 400 }
      );
    }

    const wordCount = text.trim().split(/\s+/).length;
    if (wordCount < 50) {
      return NextResponse.json(
        { error: "Text is too short. Please provide at least 50 words for accurate detection." },
        { status: 400 }
      );
    }

    // Write text to temp file
    await mkdir(UPLOAD_DIR, { recursive: true });
    tempPath = join(UPLOAD_DIR, `${randomUUID()}.txt`);
    await writeFile(tempPath, text, "utf-8");

    // Run provenance detect
    const { stdout, stderr } = await execFileAsync(PROVENANCE_BIN, [
      "detect",
      "--file",
      tempPath,
      "--format",
      "json",
    ], {
      timeout: 30000,
      maxBuffer: 10 * 1024 * 1024,
      env: { ...process.env, RUST_LOG: "warn" },
    });

    // Strip any log lines before JSON
    const jsonStart = stdout.indexOf("{");
    if (jsonStart === -1) {
      return NextResponse.json(
        { error: "No detection output", details: stderr || stdout.slice(0, 500) },
        { status: 500 }
      );
    }

    const result = JSON.parse(stdout.slice(jsonStart));

    return NextResponse.json({
      word_count: wordCount,
      result,
    });
  } catch (err: unknown) {
    const error = err as Error & { code?: string; stderr?: string };
    console.error("Detection error:", error);
    return NextResponse.json(
      { error: "Detection failed", details: error.stderr || error.message },
      { status: 500 }
    );
  } finally {
    if (tempPath) {
      unlink(tempPath).catch(() => {});
    }
  }
}

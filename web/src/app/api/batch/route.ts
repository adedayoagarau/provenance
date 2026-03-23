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
const MAX_FILE_SIZE = 100 * 1024 * 1024;

export async function POST(request: NextRequest) {
  const tempPaths: string[] = [];

  try {
    const formData = await request.formData();
    const files = formData.getAll("files") as File[];

    if (!files.length) {
      return NextResponse.json({ error: "No files provided" }, { status: 400 });
    }

    await mkdir(UPLOAD_DIR, { recursive: true });

    const results = [];

    for (const file of files) {
      if (file.size > MAX_FILE_SIZE) {
        results.push({
          file_name: file.name,
          error: "File exceeds 100MB limit",
        });
        continue;
      }

      const ext = "." + file.name.split(".").pop()?.toLowerCase();
      if (!ACCEPTED_EXTENSIONS.includes(ext)) {
        results.push({
          file_name: file.name,
          error: `Unsupported file type: ${ext}`,
        });
        continue;
      }

      const safeFileName = `${randomUUID()}${ext}`;
      const tempPath = join(UPLOAD_DIR, safeFileName);
      tempPaths.push(tempPath);

      try {
        const buffer = Buffer.from(await file.arrayBuffer());
        await writeFile(tempPath, buffer);

        const { stdout } = await execFileAsync(PROVENANCE_BIN, [
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

        const result = JSON.parse(stdout);
        results.push({
          file_name: file.name,
          file_size: file.size,
          result,
        });
      } catch (err: unknown) {
        const error = err as Error & { stderr?: string };
        results.push({
          file_name: file.name,
          error: error.stderr || error.message,
        });
      }
    }

    return NextResponse.json({ results });
  } catch (err: unknown) {
    const error = err as Error;
    return NextResponse.json(
      { error: "Batch analysis failed", details: error.message },
      { status: 500 }
    );
  } finally {
    for (const p of tempPaths) {
      unlink(p).catch(() => {});
    }
  }
}

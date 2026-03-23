import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Provenance — Document Analysis",
  description: "Forensic authorship verification platform",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-bg-primary">
        <header className="border-b border-border px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="font-mono text-sm font-semibold tracking-wider text-text-primary uppercase">
              Provenance
            </span>
            <span className="text-text-tertiary text-xs font-mono">
              v0.1.0
            </span>
          </div>
          <nav className="flex gap-6">
            <a
              href="/"
              className="text-xs font-mono text-text-secondary hover:text-text-primary transition-colors uppercase tracking-wide"
            >
              Analyze
            </a>
            <a
              href="/batch"
              className="text-xs font-mono text-text-secondary hover:text-text-primary transition-colors uppercase tracking-wide"
            >
              Batch
            </a>
          </nav>
        </header>
        <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
      </body>
    </html>
  );
}

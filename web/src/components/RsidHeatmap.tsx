"use client";

import { RsidAnalysis } from "@/lib/types";

function hashToHue(rsid: string): number {
  let hash = 0;
  for (let i = 0; i < rsid.length; i++) {
    hash = rsid.charCodeAt(i) + ((hash << 5) - hash);
  }
  return Math.abs(hash) % 360;
}

function rsidColor(rsid: string, opacity: number = 0.7): string {
  const hue = hashToHue(rsid);
  return `hsla(${hue}, 60%, 55%, ${opacity})`;
}

export default function RsidHeatmap({ rsid }: { rsid: RsidAnalysis }) {
  if (!rsid.paragraph_rsids || rsid.paragraph_rsids.length === 0) {
    return (
      <div className="text-xs text-text-tertiary font-mono">
        No RSID data available
      </div>
    );
  }

  const paragraphs = rsid.paragraph_rsids;
  const totalParagraphs = paragraphs.length;

  // Find the largest contiguous block for each RSID
  const rsidCounts: Record<string, number> = {};
  for (const p of paragraphs) {
    rsidCounts[p.rsid] = (rsidCounts[p.rsid] || 0) + 1;
  }

  // Find the dominant RSID (largest single block)
  const dominantRsid = Object.entries(rsidCounts).sort((a, b) => b[1] - a[1])[0]?.[0];

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <div className="text-xs text-text-secondary font-mono uppercase tracking-wide">
          Paragraph RSID Distribution
        </div>
        <div className="flex items-center gap-4 text-xs font-mono text-text-tertiary">
          <span>{totalParagraphs} paragraphs</span>
          <span>{rsid.unique_rsids} unique RSIDs</span>
          <span>diversity: {(rsid.rsid_diversity_ratio * 100).toFixed(1)}%</span>
        </div>
      </div>

      {/* Horizontal bar chart - each paragraph is a thin vertical stripe */}
      <div className="w-full h-10 flex border border-border overflow-hidden">
        {paragraphs.map((p, i) => {
          const isLargestBlock = p.rsid === dominantRsid && rsidCounts[dominantRsid] > totalParagraphs * 0.3;
          return (
            <div
              key={i}
              title={`¶${p.paragraph_index}: RSID ${p.rsid} (${p.word_count} words)`}
              className="h-full"
              style={{
                flex: `${Math.max(p.word_count, 1)} 0 0`,
                backgroundColor: rsidColor(p.rsid, isLargestBlock ? 0.9 : 0.6),
                borderRight:
                  i < paragraphs.length - 1 && paragraphs[i + 1]?.rsid !== p.rsid
                    ? "1px solid rgba(15,17,23,0.8)"
                    : "none",
              }}
            />
          );
        })}
      </div>

      {/* Largest block callout */}
      {rsid.largest_block_percentage > 30 && (
        <div className="mt-2 text-xs font-mono text-status-amber">
          Largest single-RSID block: {rsid.largest_single_rsid_block} paragraphs (
          {rsid.largest_block_percentage.toFixed(1)}% of document)
        </div>
      )}

      {/* Cluster breakdown */}
      {rsid.rsid_clusters && rsid.rsid_clusters.length > 0 && (
        <div className="mt-3 grid grid-cols-2 md:grid-cols-4 gap-2">
          {rsid.rsid_clusters
            .sort((a, b) => b.paragraphs.length - a.paragraphs.length)
            .slice(0, 8)
            .map((cluster, i) => (
              <div
                key={i}
                className="border border-border px-2 py-1.5 flex items-center gap-2"
              >
                <div
                  className="w-3 h-3 shrink-0"
                  style={{ backgroundColor: rsidColor(cluster.rsid, 0.8) }}
                />
                <div className="text-xs font-mono text-text-secondary truncate">
                  {cluster.paragraphs.length}¶ &middot; {cluster.total_words}w
                  {cluster.is_contiguous && (
                    <span className="text-text-tertiary"> &middot; contig</span>
                  )}
                </div>
              </div>
            ))}
        </div>
      )}
    </div>
  );
}

// Types matching the Rust CLI JSON output (UnifiedScore)

export interface AnalysisResult {
  forensic_report: ForensicReport;
  analysis_result: TextAnalysis;
  comparison: ComparisonResult | null;
  overall_confidence: number | null;
  audit: AuditTrail;
  register: RegisterClassification | null;
  baseline_report: RegisterBaselineReport | null;
  acs: AuthorshipConfidenceScore | null;
  pii: ProcessIntegrityIndex | null;
  anomaly_report: AnomalyReport | null;
  evaluator_report: EvaluatorReport | null;
  explained_decision: ExplainedDecision | null;
}

export interface AuditTrail {
  software_version: string;
  input_file_hash: string;
  input_file_path: string;
  timestamp: string;
  config_summary: string;
  feature_set: string;
  feature_count: number;
}

export interface ForensicReport {
  file_path: string;
  file_size: number;
  format_detected: string;
  metadata: FileMetadata | null;
  document_metadata: DocumentMetadata | null;
  tampering: TamperingReport | null;
  timeline: TimelineReport | null;
  docx_profile: DocumentConstructionProfile | null;
}

export interface FileMetadata {
  [key: string]: unknown;
}

export interface DocumentMetadata {
  author: string | null;
  title: string | null;
  creator: string | null;
  last_modified_by: string | null;
  revision_count: number | null;
  reported_word_count: number | null;
  creation_date: string | null;
  modification_date: string | null;
  application: string | null;
  custom: Record<string, string>;
}

export interface TamperingReport {
  risk_score: number;
  findings: TamperingFinding[];
}

export interface TamperingFinding {
  category: string;
  severity: string;
  description: string;
}

export interface TimelineReport {
  entries: TimelineEntry[];
  anomalies: TimelineAnomaly[];
}

export interface TimelineEntry {
  event_type: string;
  timestamp: string;
  source: string;
}

export interface TimelineAnomaly {
  description: string;
  severity: string;
}

export interface DocumentConstructionProfile {
  metadata: DocumentMetadata | null;
  rsid_analysis: RsidAnalysis | null;
  formatting_analysis: FormattingAnalysis | null;
  structural_forensics: StructuralForensics | null;
  construction_pattern: ConstructionPattern;
  anomalies: ForensicAnomaly[];
  process_integrity_score: number;
  assessment_text: string;
  editing_velocity: number | null;
  saves_per_hour: number | null;
  creation_to_modification_hours: number | null;
  words_per_save: number | null;
}

export type ConstructionPattern = "Organic" | "BulkInsertion" | "Hybrid" | "Insufficient";

export interface RsidAnalysis {
  total_paragraphs: number;
  unique_rsids: number;
  rsid_diversity_ratio: number;
  largest_single_rsid_block: number;
  largest_block_percentage: number;
  revision_scatter: number;
  paragraph_rsids: ParagraphRsid[];
  rsid_clusters: RsidCluster[];
}

export interface ParagraphRsid {
  paragraph_index: number;
  rsid: string;
  word_count: number;
}

export interface RsidCluster {
  rsid: string;
  paragraphs: number[];
  total_words: number;
  is_contiguous: boolean;
}

export interface FormattingAnalysis {
  [key: string]: unknown;
}

export interface StructuralForensics {
  [key: string]: unknown;
}

export interface ForensicAnomaly {
  anomaly_type: string;
  location: [number, number] | null;
  severity: "Low" | "Medium" | "High";
  description: string;
}

export interface TextAnalysis {
  lexical: LexicalProfile;
  syntactic: SyntacticProfile;
  semantic: SemanticProfile;
  stylometric: StylometricProfile;
  function_words: FunctionWordProfile;
  ngrams: NgramProfile;
}

export interface LexicalProfile {
  word_count: number;
  unique_words: number;
  type_token_ratio: number;
  avg_word_length: number;
  mattr: number | null;
  yules_k: number | null;
  honores_r: number | null;
  brunets_w: number | null;
  hapax_legomena: number;
  dis_legomena: number;
  [key: string]: unknown;
}

export interface SyntacticProfile {
  sentence_count: number;
  avg_sentence_length: number;
  sentence_length_std: number;
  passive_voice_ratio: number;
  [key: string]: unknown;
}

export interface SemanticProfile {
  flesch_kincaid_grade: number;
  gunning_fog: number;
  coleman_liau: number;
  automated_readability: number;
  discourse_marker_density: number;
  [key: string]: unknown;
}

export interface StylometricProfile {
  contraction_ratio: number;
  hedge_word_density: number;
  intensifier_density: number;
  [key: string]: unknown;
}

export interface FunctionWordProfile {
  function_word_ratio: number;
  function_word_diversity: number;
  [key: string]: unknown;
}

export interface NgramProfile {
  [key: string]: unknown;
}

export interface RegisterClassification {
  register: string;
  confidence: number;
  label: string;
}

export interface RegisterBaselineReport {
  register_label: string;
  comparisons: BaselineComparison[];
  anomalous_count: number;
  atypical_count: number;
  expected_count: number;
}

export interface BaselineComparison {
  metric_name: string;
  actual_value: number;
  expected_mean: number;
  expected_stddev: number;
  z_score: number;
  assessment: "Expected" | "Atypical" | "Anomalous";
  explanation: string;
}

export interface AuthorshipConfidenceScore {
  score: number;
  margin: number;
  score_low: number;
  score_high: number;
  evidence_sources: EvidenceSources;
  weights: ScoreWeights;
  components: ScoreComponents;
}

export interface EvidenceSources {
  has_forensics: boolean;
  has_stylometric_profile: boolean;
  has_text_analysis: boolean;
  has_register_baselines: boolean;
}

export interface ScoreWeights {
  forensics_weight: number;
  stylometric_weight: number;
  text_analysis_weight: number;
}

export interface ScoreComponents {
  forensics_score: number | null;
  stylometric_score: number | null;
  text_analysis_score: number;
}

export interface ProcessIntegrityIndex {
  score: number;
  level: "Strong" | "Moderate" | "Limited" | "Insufficient";
  guidance: string;
  components: IntegrityComponents;
}

export interface IntegrityComponents {
  metadata_completeness: number;
  rsid_richness: number;
  formatting_quality: number;
  revision_evidence: number;
  additional_evidence: number;
}

export interface AnomalyReport {
  flags: AnomalyFlag[];
  high_count: number;
  medium_count: number;
  low_count: number;
  summary: string;
}

export interface AnomalyFlag {
  anomaly_type: string;
  location: AnomalyLocation;
  severity: "Low" | "Medium" | "High";
  description: string;
  recommended_action: string;
  contributing_signals: string[];
}

export type AnomalyLocation =
  | "DocumentLevel"
  | { ParagraphRange: { start: number; end: number } }
  | { Section: string };

export interface EvaluatorReport {
  executive_summary: string;
  reliability_disclosure: ReliabilityDisclosure;
  register_context: RegisterContext;
  authorship_assessment: AuthorshipAssessment;
  integrity_assessment: IntegrityAssessment;
  anomaly_narrative: AnomalyNarrative;
  recommended_actions: string[];
}

export interface ReliabilityDisclosure {
  header: string;
  points: string[];
  evidence_basis: string;
}

export interface RegisterContext {
  summary: string;
  implication: string;
}

export interface AuthorshipAssessment {
  summary: string;
  score_display: string;
  confidence_interval: string;
  interpretation: string;
  caveats: string[];
}

export interface IntegrityAssessment {
  summary: string;
  level: string;
  guidance: string;
}

export interface AnomalyNarrative {
  summary: string;
  details: string[];
  framing_note: string;
}

export interface ExplainedDecision {
  [key: string]: unknown;
}

export interface ComparisonResult {
  [key: string]: unknown;
}

// Batch results
export interface BatchResult {
  file_name: string;
  construction_pattern: ConstructionPattern | null;
  acs_score: number | null;
  acs_margin: number | null;
  pii_score: number | null;
  anomaly_count: number;
  register: string | null;
  full_result: AnalysisResult;
}

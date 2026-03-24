use clap::Parser;
use provenance::scoring::engine::OutputFormat;
use provenance::utils;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "provenance", version, about = "Forensic authorship verification platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Analyze a document for authorship verification
    Analyze {
        /// Path to the document to analyze
        #[arg(short, long)]
        file: String,

        /// Path to an author profile to compare against
        #[arg(short, long)]
        profile: Option<String>,

        /// Output format: text, json, html
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Build an author profile from verified writing samples
    Profile {
        /// Directory containing verified writing samples
        #[arg(short, long)]
        samples: String,

        /// Name for the author profile
        #[arg(short, long)]
        name: String,
    },

    /// Verify file integrity and metadata
    Forensics {
        /// Path to the file to examine
        #[arg(short, long)]
        file: String,

        /// Output format: text, json, html
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Deep forensic analysis of a .docx file (RSID, formatting, structure, construction profile)
    DocxForensics {
        /// Path to the .docx file to examine
        #[arg(short, long)]
        file: String,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Batch-analyze a directory of documents
    Batch {
        /// Directory containing documents to analyze
        #[arg(short, long)]
        dir: String,

        /// Path to an author profile to compare against (optional)
        #[arg(short, long)]
        profile: Option<String>,

        /// Output format: text, json
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Rank multiple author candidates against a document
    Rank {
        /// Path to the document to analyze
        #[arg(short, long)]
        file: String,

        /// Paths to author profile files (at least one required)
        #[arg(short, long, num_args = 1..)]
        profiles: Vec<String>,

        /// Output format: text, json, html
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Generate an Ed25519 signing keypair for certificate generation
    GenerateKey {
        /// Directory to save the keypair files
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// Generate a tamper-evident Provenance certificate for a document
    GenerateCertificate {
        /// Path to the document to certify
        #[arg(short, long)]
        file: String,

        /// Path to the Ed25519 signing key file
        #[arg(short, long)]
        key: String,

        /// Path to an author profile to compare against (optional)
        #[arg(short, long)]
        profile: Option<String>,

        /// Output path for the certificate (default: <filename>.provenance.json)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Verify a Provenance certificate's integrity and authenticity
    VerifyCertificate {
        /// Path to the certificate file
        #[arg(short, long)]
        certificate: String,

        /// Path to the original document to verify against (optional)
        #[arg(short, long)]
        document: Option<String>,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Import a capture session from a plugin and compute process metrics
    ImportCapture {
        /// Path to a capture session JSON file, or directory of session files
        #[arg(short, long)]
        file: String,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Watch a Scrivener project directory for changes and capture events
    CaptureWatch {
        /// Path to the .scriv project directory
        #[arg(short, long)]
        project: String,

        /// Poll interval in seconds
        #[arg(long, default_value = "10")]
        interval: u64,

        /// Output path for the capture session
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Run adversarial robustness assessment (evasion scenarios + robustness matrix)
    AdversarialTest {
        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Detect AI-generated writing in a document (no author profile needed)
    Detect {
        /// Path to the document to analyze
        #[arg(short, long)]
        file: String,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Detect humanizer tool artifacts in a document
    DetectHumanizer {
        /// Path to the document to analyze
        #[arg(short, long)]
        file: String,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Generate a bias audit report
    BiasAudit {
        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Start the Provenance API server
    #[cfg(feature = "server")]
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(long, default_value = "3000")]
        port: u16,

        /// Require API key authentication
        #[arg(long)]
        require_auth: bool,

        /// Rate limit (requests per minute, 0 = unlimited)
        #[arg(long, default_value = "60")]
        rate_limit: u32,

        /// Maximum request body size in MB
        #[arg(long, default_value = "100")]
        max_body_mb: usize,
    },
}

fn main() -> anyhow::Result<()> {
    utils::logging::init();

    let cli = Cli::parse();
    info!("Provenance v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Analyze { file, profile, format } => {
            info!(file = %file, "Starting document analysis");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::analyze_with_format(&file, profile.as_deref(), output_format)?;
            println!("{report}");
        }
        Commands::Profile { samples, name } => {
            info!(name = %name, "Building author profile");
            let profile = provenance::build_profile(&samples, &name)?;
            println!("Profile created: {profile}");
        }
        Commands::Forensics { file, format } => {
            info!(file = %file, "Running file forensics");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::run_forensics_with_format(&file, output_format)?;
            println!("{report}");
        }
        Commands::DocxForensics { file, format } => {
            info!(file = %file, "Running DOCX forensic analysis");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::run_docx_forensics(&file, output_format)?;
            println!("{report}");
        }
        Commands::Batch { dir, profile, format } => {
            info!(dir = %dir, "Running batch analysis");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::batch_analyze(&dir, profile.as_deref(), output_format)?;
            println!("{report}");
        }
        Commands::Rank { file, profiles, format } => {
            info!(file = %file, candidates = profiles.len(), "Ranking author candidates");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::rank_candidates_with_format(&file, &profiles, output_format)?;
            println!("{report}");
        }
        Commands::GenerateKey { output } => {
            info!(output = %output, "Generating Ed25519 signing keypair");
            let (private_path, public_path) = provenance::generate_signing_key(&output)?;
            println!("Signing key saved: {private_path}");
            println!("Public key saved:  {public_path}");
            println!("\nKeep the signing key secure. Distribute the public key for verification.");
        }
        Commands::GenerateCertificate { file, key, profile, output } => {
            info!(file = %file, "Generating Provenance certificate");
            let cert_path = provenance::generate_certificate(
                &file,
                &key,
                profile.as_deref(),
                output.as_deref(),
            )?;
            println!("Certificate saved: {cert_path}");
        }
        Commands::VerifyCertificate { certificate, document, format } => {
            info!(certificate = %certificate, "Verifying Provenance certificate");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::verify_certificate(
                &certificate,
                document.as_deref(),
                output_format,
            )?;
            println!("{report}");
        }
        Commands::ImportCapture { file, format } => {
            info!(file = %file, "Importing capture session");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let path = std::path::Path::new(&file);
            let report = if path.is_dir() {
                provenance::import_capture_dir(&file, output_format)?
            } else {
                provenance::import_capture(&file, output_format)?
            };
            println!("{report}");
        }
        Commands::CaptureWatch { project, interval, output } => {
            info!(project = %project, "Starting Scrivener capture watch");
            let project_path = std::path::Path::new(&project);

            println!("Watching Scrivener project: {project}");
            println!("Poll interval: {interval}s");
            println!("Press Ctrl+C to stop.\n");

            let mut session = provenance::capture::session::CaptureSession::new(
                &format!("scriv-{}", uuid_hex()),
                project_path.file_name().and_then(|n| n.to_str()).unwrap_or("project"),
                provenance::capture::events::CaptureSource::Scrivener,
                &now_iso8601(),
            );

            let mut state = provenance::capture::scrivener::scan_project(project_path)?;
            let start = std::time::Instant::now();

            loop {
                std::thread::sleep(std::time::Duration::from_secs(interval));

                let new_state = provenance::capture::scrivener::scan_project(project_path)?;
                let elapsed = start.elapsed().as_millis() as u64;
                let events = provenance::capture::scrivener::diff_states(
                    &state,
                    &new_state,
                    elapsed,
                    session.event_count() as u64,
                );

                if !events.is_empty() {
                    println!("[{:.1}s] {} change(s) detected", elapsed as f64 / 1000.0, events.len());
                    for event in events {
                        session.push_event(event);
                    }
                }

                state = new_state;

                // Auto-save session periodically
                if session.event_count() % 50 == 0 && session.event_count() > 0 {
                    let save_path = output
                        .as_deref()
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| std::path::PathBuf::from(format!("{}_capture.json", session.document_name)));
                    provenance::capture::session::save_session(&session, &save_path)?;
                }
            }
        }
        Commands::AdversarialTest { format } => {
            info!("Running adversarial robustness assessment");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::adversarial_report(output_format)?;
            println!("{report}");
        }
        Commands::Detect { file, format } => {
            info!(file = %file, "Running AI detection analysis");
            let result = provenance::detection::detect_file(&file)?;
            let output = match format.as_str() {
                "json" => provenance::detection::format_json(&result),
                _ => provenance::detection::format_text(&result),
            };
            println!("{output}");
        }
        Commands::DetectHumanizer { file, format } => {
            info!(file = %file, "Running humanizer detection");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::detect_humanizer(&file, output_format)?;
            println!("{report}");
        }
        Commands::BiasAudit { format } => {
            info!("Generating bias audit report");
            let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let report = provenance::bias_audit_report(output_format)?;
            println!("{report}");
        }
        #[cfg(feature = "server")]
        Commands::Serve {
            host,
            port,
            require_auth,
            rate_limit,
            max_body_mb,
        } => {
            let config = provenance::api::server::ServerConfig {
                host,
                port,
                max_body_size: max_body_mb * 1024 * 1024,
                default_rate_limit_rpm: rate_limit,
                max_concurrent_jobs: 10,
                auth: provenance::api::auth::ApiKeyConfig {
                    keys: std::collections::HashMap::new(),
                    require_auth,
                },
            };

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(provenance::api::server::start(config))
                .map_err(|e| anyhow::anyhow!("Server error: {e}"))?;
        }
    }

    Ok(())
}

fn uuid_hex() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    provenance::scoring::engine::format_epoch_public(secs)
}

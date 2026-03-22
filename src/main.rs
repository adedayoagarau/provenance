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
    }

    Ok(())
}

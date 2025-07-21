use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use failure::{Error, format_err};
use log::info;
use tokio::{
    fs,
    process::Command,
    sync::RwLock,
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle,
};

use crate::config::Handbrake;
use crate::disc::{DiscMetadata, DiscType};

#[derive(Debug, Clone)]
pub struct JobStatus {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub status: String,
    pub started_at: String,
    pub progress: f32,
}

#[derive(Clone)]
pub struct HandbrakeProcess {
    tx: UnboundedSender<Job>,
    pub jobs: Arc<RwLock<HashMap<String, JobStatus>>>,
}

#[derive(Debug)]
struct Job {
    id: String,
    src: PathBuf,
    dest: PathBuf,
}

impl HandbrakeProcess {
    pub fn new(config: Handbrake) -> (HandbrakeProcess, JoinHandle<Result<(), Error>>) {
        let (tx, mut rx) = unbounded_channel();
        let jobs = Arc::new(RwLock::new(HashMap::<String, JobStatus>::new()));
        let jobs_clone = jobs.clone();

        let handle = tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                let job: Job = job;

                // Update job status to "Processing"
                {
                    let mut jobs_map = jobs_clone.write().await;
                    if let Some(job_status) = jobs_map.get_mut(&job.id) {
                        job_status.status = "Processing".to_string();
                        job_status.progress = 0.0;
                    }
                }

                match handbrake(&config, &job.src, &job.dest, &job.id, &jobs_clone).await {
                    Ok(_) => {
                        // Mark job as completed
                        let mut jobs_map = jobs_clone.write().await;
                        if let Some(job_status) = jobs_map.get_mut(&job.id) {
                            job_status.status = "Completed".to_string();
                            job_status.progress = 1.0;
                        }
                    }
                    Err(e) => {
                        // Mark job as failed
                        let mut jobs_map = jobs_clone.write().await;
                        if let Some(job_status) = jobs_map.get_mut(&job.id) {
                            job_status.status = format!("Failed: {e}");
                            job_status.progress = 0.0;
                        }
                    }
                }
            }

            info!("exiting handbrake process");

            Ok(())
        });

        (HandbrakeProcess { tx, jobs }, handle)
    }

    pub async fn queue(&self, src: PathBuf, dest: PathBuf) -> Result<(), Error> {
        let job_id = format!("{}", uuid::Uuid::new_v4());
        let job_status = JobStatus {
            id: job_id.clone(),
            source: src.display().to_string(),
            destination: dest.display().to_string(),
            status: "Queued".to_string(),
            started_at: chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
            progress: 0.0,
        };

        // Add job to tracking
        {
            let mut jobs_map = self.jobs.write().await;
            jobs_map.insert(job_id.clone(), job_status);
        }

        self.tx.send(Job {
            id: job_id,
            src,
            dest,
        })?;

        Ok(())
    }

    pub async fn get_active_jobs(&self) -> Vec<JobStatus> {
        let jobs_map = self.jobs.read().await;
        jobs_map
            .values()
            .filter(|job| job.status != "Completed")
            .cloned()
            .collect()
    }

    pub async fn get_queue_size(&self) -> usize {
        let jobs_map = self.jobs.read().await;
        jobs_map
            .values()
            .filter(|job| job.status == "Queued")
            .count()
    }
}

async fn handbrake(
    config: &Handbrake,
    src: &Path,
    dest: &Path,
    job_id: &str,
    jobs: &Arc<RwLock<HashMap<String, JobStatus>>>,
) -> Result<(), Error> {
    let disc_meta: DiscMetadata = toml::from_slice(&fs::read(src.join("meta.toml")).await?)?;

    let args = match disc_meta.disc_type {
        DiscType::Dvd => &config.dvd,
        DiscType::BluRay => &config.bluray,
        _ => unimplemented!(),
    };

    let dest = dest.join(src.file_name().unwrap());

    fs::create_dir_all(&dest).await?;

    let mut files = fs::read_dir(src).await?;

    while let Ok(Some(entry)) = files.next_entry().await {
        if "toml" == entry.path().extension().unwrap().to_str().unwrap() {
            continue;
        }

        let path = entry.path();

        if path.is_file() {
            let mut output_file = path.clone();
            output_file.set_extension(&args.extension);

            let source_file = path
                .to_str()
                .ok_or_else(|| format_err!("path is not a valid string: {:?}", path))?;

            let dest_file = dest.join(
                output_file
                    .file_name()
                    .ok_or_else(|| format_err!("path is not a valid string: {:?}", dest))?,
            );
            let dest_file = dest_file
                .to_str()
                .ok_or_else(|| format_err!("path is not a valid string: {:?}", dest_file))?;

            // Update progress to indicate file processing started
            {
                let mut jobs_map = jobs.write().await;
                if let Some(job_status) = jobs_map.get_mut(job_id) {
                    job_status.status = format!("Processing: {source_file}");
                    job_status.progress = 0.5; // Rough estimate
                }
            }

            let mut child = Command::new("HandBrakeCLI")
                .args([
                    "-i",
                    source_file,
                    "-o",
                    dest_file,
                    "--preset",
                    &args.preset,
                    "--subtitle",
                    "scan",
                    "-F",
                ])
                .args(&args.args)
                .spawn()
                .expect("failed to execute process");

            let status = child.wait().await?;

            if !status.success() {
                return Err(format_err!(
                    "error code {:?} from handbrake, stopping process",
                    status.code()
                ));
            }
        }
    }

    info!(
        "finished handbake processing into directory {}",
        dest.to_str().unwrap()
    );

    let mut files = fs::read_dir(src).await?;

    if config.delete_on_complete {
        while let Ok(Some(entry)) = files.next_entry().await {
            fs::remove_file(entry.path()).await?;
        }
    }

    Ok(())
}

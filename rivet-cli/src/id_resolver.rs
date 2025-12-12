//! ID resolver module
//!
//! Handles resolution of UUID prefixes to full UUIDs by querying the API.
//! This allows users to specify short, unambiguous prefixes instead of full UUIDs.

use anyhow::{Context, Result, anyhow};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::types::IdOrPrefix;

/// Resolve a pipeline ID or prefix to a full UUID
///
/// If the input is already a full UUID, returns it immediately.
/// Otherwise, fetches all pipelines and finds the one matching the prefix.
///
/// # Arguments
/// * `client` - The API client to use for fetching pipelines
/// * `id_or_prefix` - The ID or prefix to resolve
///
/// # Returns
/// The resolved UUID
///
/// # Errors
/// Returns an error if:
/// - No pipeline matches the prefix
/// - Multiple pipelines match the prefix (ambiguous)
/// - API call fails
pub async fn resolve_pipeline_id(client: &ApiClient, id_or_prefix: &IdOrPrefix) -> Result<Uuid> {
    // If it's already a full UUID, return it
    if let Some(uuid) = id_or_prefix.as_uuid() {
        return Ok(uuid);
    }

    let prefix = id_or_prefix.as_str().to_lowercase();

    // Fetch all pipelines
    let pipelines = client
        .list_pipelines()
        .await
        .context("Failed to fetch pipelines for ID resolution")?;

    // Find matching pipelines
    let matches: Vec<_> = pipelines
        .iter()
        .filter(|p| p.id.to_string().to_lowercase().starts_with(&prefix))
        .collect();

    match matches.len() {
        0 => Err(anyhow!(
            "No pipeline found with ID starting with '{}'",
            prefix
        )),
        1 => Ok(matches[0].id),
        _ => {
            let ids: Vec<String> = matches.iter().map(|p| p.id.to_string()).collect();
            Err(anyhow!(
                "Ambiguous prefix '{}' matches multiple pipelines: {}",
                prefix,
                ids.join(", ")
            ))
        }
    }
}

/// Resolve a job ID or prefix to a full UUID
///
/// If the input is already a full UUID, returns it immediately.
/// Otherwise, fetches all scheduled jobs and finds the one matching the prefix.
///
/// # Arguments
/// * `client` - The API client to use for fetching jobs
/// * `id_or_prefix` - The ID or prefix to resolve
///
/// # Returns
/// The resolved UUID
///
/// # Errors
/// Returns an error if:
/// - No job matches the prefix
/// - Multiple jobs match the prefix (ambiguous)
/// - API call fails
pub async fn resolve_job_id(client: &ApiClient, id_or_prefix: &IdOrPrefix) -> Result<Uuid> {
    // If it's already a full UUID, return it
    if let Some(uuid) = id_or_prefix.as_uuid() {
        return Ok(uuid);
    }

    let prefix = id_or_prefix.as_str().to_lowercase();

    // Fetch all scheduled jobs
    let jobs = client
        .list_scheduled_jobs()
        .await
        .context("Failed to fetch jobs for ID resolution")?;

    // Find matching jobs
    let matches: Vec<_> = jobs
        .iter()
        .filter(|j| j.id.to_string().to_lowercase().starts_with(&prefix))
        .collect();

    match matches.len() {
        0 => Err(anyhow!("No job found with ID starting with '{}'", prefix)),
        1 => Ok(matches[0].id),
        _ => {
            let ids: Vec<String> = matches.iter().map(|j| j.id.to_string()).collect();
            Err(anyhow!(
                "Ambiguous prefix '{}' matches multiple jobs: {}",
                prefix,
                ids.join(", ")
            ))
        }
    }
}

/// Resolve a job ID or prefix within a specific pipeline
///
/// Similar to `resolve_job_id` but only searches within jobs of a specific pipeline.
///
/// # Arguments
/// * `client` - The API client to use for fetching jobs
/// * `pipeline_id` - The pipeline to search within
/// * `id_or_prefix` - The job ID or prefix to resolve
///
/// # Returns
/// The resolved UUID
///
/// # Errors
/// Returns an error if:
/// - No job matches the prefix in this pipeline
/// - Multiple jobs match the prefix (ambiguous)
/// - API call fails
pub async fn resolve_job_id_in_pipeline(
    client: &ApiClient,
    pipeline_id: Uuid,
    id_or_prefix: &IdOrPrefix,
) -> Result<Uuid> {
    // If it's already a full UUID, return it
    if let Some(uuid) = id_or_prefix.as_uuid() {
        return Ok(uuid);
    }

    let prefix = id_or_prefix.as_str().to_lowercase();

    // Fetch jobs for this pipeline
    let jobs = client
        .list_jobs_by_pipeline(pipeline_id)
        .await
        .context("Failed to fetch pipeline jobs for ID resolution")?;

    // Find matching jobs
    let matches: Vec<_> = jobs
        .iter()
        .filter(|j| j.id.to_string().to_lowercase().starts_with(&prefix))
        .collect();

    match matches.len() {
        0 => Err(anyhow!(
            "No job found with ID starting with '{}' in pipeline {}",
            prefix,
            pipeline_id
        )),
        1 => Ok(matches[0].id),
        _ => {
            let ids: Vec<String> = matches.iter().map(|j| j.id.to_string()).collect();
            Err(anyhow!(
                "Ambiguous prefix '{}' matches multiple jobs in pipeline {}: {}",
                prefix,
                pipeline_id,
                ids.join(", ")
            ))
        }
    }
}

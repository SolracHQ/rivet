# Orchestrator

Rivet Orchestrator is the heart of the Rivet CI/CD system. It manages the lifecycle of CI/CD jobs, stores pipeline definitions, and coordinates job execution across multiple Rivet Runners.

## HTTP API

Important: All runtime endpoints are exposed under the `/api` prefix (e.g., `/api/health`).

The Orchestrator exposes the following endpoints (method + path) for health checks, runner registration/heartbeats, pipelines, jobs, and logs:

- Health
  - `GET /api/health` — Health check endpoint.

- Runner endpoints (for background runner integration)
  - `POST /api/runners/register` — Register runner capabilities. Request: `RegisterRequest` (runner_id, capabilities). Response: 200 OK.
  - `POST /api/runners/{runner_id}/heartbeat` — Send a heartbeat for the runner. Response: 200 OK.

- Job endpoints (runner-facing)
  - `GET /api/jobs/scheduled?runner_id={runner_id}` — Fetch scheduled jobs filtered by runner capabilities (via `runner_id` param). Response: `Vec<Job>`.
  - `POST /api/jobs/{job_id}/claim` — Claim a job for execution. Request: `ClaimJobRequest` ({ runner_id }). Response: `JobExecutionInfo` (job_id, pipeline_id, pipeline_source, parameters).
  - `PUT /api/jobs/{job_id}/status` — Update status for a job (e.g., Running). Request: `UpdateStatusRequest` ({ status }). Response: 200 OK / 204 No Content.
  - `POST /api/jobs/{job_id}/complete` — Mark a job as complete and send the result. Request: `CompleteJobRequest` ({ result: JobResult }). Response: 200 OK / 204 No Content.
  - `POST /api/jobs/{job_id}/logs` — Add log entries to a job. Request: `SendLogsRequest` ({ entries: Vec<LogEntry> }). Response: 201 Created.
  - `GET /api/jobs/{job_id}/logs` — Get logs for a job. Response: `Vec<LogEntry>`.
  - `GET /api/jobs/{job_id}` — Get job details by ID. Response: `Job`.
  - `GET /api/jobs/pipeline/{pipeline_id}` — List jobs related to a specific pipeline. Response: `Vec<JobDto>`.

- Pipeline endpoints (CLI/Admin-facing)
  - `POST /api/pipeline/create` — Create a new pipeline. Request: `CreatePipelineRequest`. Response: `Pipeline`.
  - `POST /api/pipeline/launch` — Create and launch a new job for a pipeline. Request: `CreateJobRequest`. Response: `Job`.
  - `GET /api/pipeline/list` — List all pipelines. Response: `Vec<PipelineDto>`.
  - `GET /api/pipeline/{id}` — Get pipeline by ID. Response: `Pipeline`.
  - `DELETE /api/pipeline/{id}` — Delete a pipeline. Response: 204 No Content.

Notes:
- Most endpoints return 200 OK with JSON bodies on success, unless noted (e.g., 204 No Content on delete, 201 Created on log append).

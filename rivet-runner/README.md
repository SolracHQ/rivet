# Rivet Runner

Stateless worker that executes pipeline jobs in secure Lua sandboxes.

Workflow:

- Poll available jobs for its capabilities
- If jobs available, reserve one
- Execute each stage in order within a Lua sandbox
- Report logs every LOG_SEND_INTERVAL or when buffer full
- Report job completion

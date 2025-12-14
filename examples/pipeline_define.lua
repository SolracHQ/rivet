-- Example pipeline using the pipeline.define() API
-- This provides better LSP support and type checking

return pipeline.define({
    name = "Build and Test Pipeline",
    description = "Demonstrates the pipeline.define() API with multiple input types and conditional stages",

    -- Define input parameters with type safety
    inputs = {
        branch = {
            type = "string",
            description = "Git branch to build",
            default = "main"
        },
        verbose = {
            type = "bool",
            description = "Enable verbose logging",
            default = false,
            required = false
        },
        parallel_jobs = {
            type = "number",
            description = "Number of parallel build jobs",
            default = 4,
            options = { 1, 2, 4, 8, 16 }
        },
        environment = {
            type = "string",
            description = "Deployment environment",
            default = "staging",
            options = { "development", "staging", "production" }
        },
        skip_tests = {
            type = "bool",
            description = "Skip test stage",
            default = false,
            required = false
        }
    },

    -- Define runner requirements
    runner = {
        { key = "os",   value = "linux" },
        { key = "arch", value = "x86_64" }
    },

    -- Required plugins (if any)
    plugins = {},

    -- Define stages
    stages = {
        {
            name = "setup",
            script = function()
                log.info("Setting up build environment...")

                local branch = input.get("branch", "main")
                local verbose = input.get("verbose", "false")
                local jobs = input.get("parallel_jobs", "4")

                log.info("Branch: " .. branch)
                log.info("Parallel jobs: " .. jobs)

                if verbose == "true" then
                    log.debug("Verbose mode enabled")
                    log.debug("Additional diagnostic information will be shown")
                end

                log.info("Setup completed")
            end
        },

        {
            name = "checkout",
            script = function()
                log.info("Checking out code...")
                local branch = input.get("branch", "main")
                log.info("Checking out branch: " .. branch)

                -- In a real pipeline, you'd use the git plugin here
                log.info("Code checkout completed")
            end
        },

        {
            name = "build",
            container = "rust:latest",
            script = function()
                log.info("Building project...")

                local jobs = input.get("parallel_jobs", "4")
                log.info("Using " .. jobs .. " parallel jobs")

                -- Example: process.run({ "cargo", "build", "--release", "-j", jobs })
                log.info("Build completed successfully")
            end
        },

        {
            name = "test",
            container = "rust:latest",
            -- Conditional stage execution
            condition = function()
                local skip = input.get("skip_tests", "false")
                return skip ~= "true"
            end,
            script = function()
                log.info("Running tests...")

                local verbose = input.get("verbose", "false")
                if verbose == "true" then
                    log.debug("Running tests in verbose mode")
                end

                -- Example: process.run({ "cargo", "test" })
                log.info("All tests passed")
            end
        },

        {
            name = "deploy",
            -- Only run for specific environments
            condition = function()
                local env = input.get("environment", "staging")
                -- Skip deploy for development environment
                return env ~= "development"
            end,
            script = function()
                local env = input.get("environment", "staging")
                log.info("Deploying to " .. env .. " environment...")

                if env == "production" then
                    log.warning("Deploying to PRODUCTION - use caution!")
                end

                log.info("Deployment completed successfully")
            end
        },

        {
            name = "notify",
            script = function()
                log.info("Sending notifications...")

                local env = input.get("environment", "staging")
                local branch = input.get("branch", "main")

                log.info("Pipeline completed for branch '" .. branch .. "' in " .. env)
            end
        }
    }
})

-- Example pipeline using the pipeline.builder() API
-- This provides a fluent builder pattern for constructing pipelines

return pipeline.builder()
    :name("Docker Build Pipeline")
    :description("Builds and pushes a Docker image using the builder pattern")

    -- Add input parameters one at a time
    :input("image_name", {
        type = "string",
        description = "Docker image name",
        required = true
    })
    :input("image_tag", {
        type = "string",
        description = "Docker image tag",
        default = "latest"
    })
    :input("registry", {
        type = "string",
        description = "Container registry URL",
        default = "docker.io"
    })
    :input("push_image", {
        type = "bool",
        description = "Push image to registry after build",
        default = true,
        required = false
    })
    :input("cache_enabled", {
        type = "bool",
        description = "Enable build cache",
        default = true,
        required = false
    })

    -- Add runner requirement tags
    :tag({ key = "os", value = "linux" })
    :tag({ key = "capability", value = "docker" })
    :tag({ key = "arch", value = "x86_64" })

    -- Add required plugins
    :plugin("docker")
    :plugin("git")

    -- Add stages one at a time
    :stage({
        name = "validate",
        script = function()
            log.info("Validating inputs...")

            local image_name = input.get("image_name")
            if not image_name or image_name == "" then
                log.error("image_name is required")
                error("Missing required input: image_name")
            end

            log.info("Image name: " .. image_name)
            log.info("Validation passed")
        end
    })

    :stage({
        name = "checkout",
        script = function()
            log.info("Checking out source code...")
            -- In a real pipeline: git.clone(...)
            log.info("Source code checked out")
        end
    })

    :stage({
        name = "build",
        container = "docker:latest",
        script = function()
            log.info("Building Docker image...")

            local image_name = input.get("image_name")
            local image_tag = input.get("image_tag", "latest")
            local full_image = image_name .. ":" .. image_tag

            log.info("Building image: " .. full_image)

            local cache_enabled = input.get("cache_enabled", "true")
            if cache_enabled == "true" then
                log.info("Cache enabled")
            else
                log.info("Cache disabled (--no-cache)")
            end

            -- In a real pipeline:
            -- process.run({
            --     "docker", "build",
            --     "-t", full_image,
            --     cache_enabled == "false" and "--no-cache" or nil,
            --     "."
            -- })

            log.info("Docker image built successfully")
        end
    })

    :stage({
        name = "test",
        container = "docker:latest",
        script = function()
            log.info("Running image tests...")

            local image_name = input.get("image_name")
            local image_tag = input.get("image_tag", "latest")
            local full_image = image_name .. ":" .. image_tag

            log.info("Testing image: " .. full_image)

            -- In a real pipeline: run container and test
            -- process.run({ "docker", "run", "--rm", full_image, "test-command" })

            log.info("Image tests passed")
        end
    })

    :stage({
        name = "push",
        container = "docker:latest",
        condition = function()
            local should_push = input.get("push_image", "true")
            return should_push == "true"
        end,
        script = function()
            log.info("Pushing image to registry...")

            local registry = input.get("registry", "docker.io")
            local image_name = input.get("image_name")
            local image_tag = input.get("image_tag", "latest")
            local full_image = registry .. "/" .. image_name .. ":" .. image_tag

            log.info("Target: " .. full_image)

            -- In a real pipeline:
            -- process.run({ "docker", "tag", image_name .. ":" .. image_tag, full_image })
            -- process.run({ "docker", "push", full_image })

            log.info("Image pushed successfully")
        end
    })

    :stage({
        name = "cleanup",
        container = "docker:latest",
        script = function()
            log.info("Cleaning up build artifacts...")

            local image_name = input.get("image_name")
            local image_tag = input.get("image_tag", "latest")

            log.info("Removing temporary images...")
            -- In a real pipeline:
            -- process.run({ "docker", "image", "prune", "-f" })

            log.info("Cleanup completed")
        end
    })

    -- Build the final pipeline definition
    :build()

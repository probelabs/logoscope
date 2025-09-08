.PHONY: deploy-site deploy-worker help

help: ## Show this help message
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'

deploy-site: ## Deploy static site to Cloudflare Pages
	npx wrangler pages deploy . --project-name=logoscope-site --commit-dirty=true

deploy-worker: ## Deploy Cloudflare Worker
	npx wrangler deploy

deploy-all: deploy-site deploy-worker ## Deploy both site and worker
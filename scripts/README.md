# GitHub Workflow Management Scripts

This directory contains scripts to help manage GitHub Actions workflow runs.

## Scripts

### delete-workflow-runs.sh

Simple script to delete all runs for a specific workflow or all workflows.

**Usage:**
```bash
# Delete all runs for a specific workflow
./scripts/delete-workflow-runs.sh CI

# Delete all runs for all workflows
./scripts/delete-workflow-runs.sh --all
```

### manage-workflow-runs.sh

Advanced script with filtering options for managing workflow runs.

**Features:**
- Filter by run status (success, failure, cancelled)
- Filter by age (delete runs older than X days)
- Keep only the latest N runs
- Dry-run mode to preview changes
- List mode to view runs without deleting

**Usage examples:**
```bash
# Delete all failed runs
./scripts/manage-workflow-runs.sh --status failure --all

# Delete runs older than 30 days
./scripts/manage-workflow-runs.sh --older-than 30 --all

# Keep only the 10 most recent runs for each workflow
./scripts/manage-workflow-runs.sh --keep-latest 10 --all

# Dry run - see what would be deleted without actually deleting
./scripts/manage-workflow-runs.sh --dry-run --all

# List all failed runs without deleting
./scripts/manage-workflow-runs.sh --list --status failure --all

# Delete failed CI runs older than 7 days
./scripts/manage-workflow-runs.sh --status failure --older-than 7 CI
```

## Prerequisites

Both scripts require the GitHub CLI (`gh`) to be installed and authenticated:

1. Install GitHub CLI:
   - macOS: `brew install gh`
   - Linux: See https://github.com/cli/cli/blob/trunk/docs/install_linux.md
   - Windows: `winget install --id GitHub.cli`

2. Authenticate:
   ```bash
   gh auth login
   ```

## Rate Limiting

GitHub has rate limits for API requests. The scripts include small delays when processing many runs to avoid hitting these limits. If you're deleting a large number of runs, the process may take some time.

## Safety Features

- Both scripts require confirmation before deleting runs
- The advanced script supports dry-run mode to preview changes
- Run IDs are shown during deletion for audit purposes

## Notes

- Deleted workflow runs cannot be recovered
- You need appropriate repository permissions to delete workflow runs
- The scripts work with both public and private repositories
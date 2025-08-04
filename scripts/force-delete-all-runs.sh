#!/bin/bash

# Force delete ALL workflow runs - cancels in-progress runs first

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Get repository info
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)

echo -e "${BLUE}Repository: $REPO${NC}"

# First, cancel all in-progress or queued runs
cancel_active_runs() {
    echo -e "${YELLOW}Cancelling active runs...${NC}"
    
    # Get all runs that are in progress or queued
    active_runs=$(gh api -X GET "/repos/$REPO/actions/runs?status=in_progress" --jq '.workflow_runs[].id')
    queued_runs=$(gh api -X GET "/repos/$REPO/actions/runs?status=queued" --jq '.workflow_runs[].id')
    
    all_active="$active_runs $queued_runs"
    
    if [ -z "$(echo $all_active | tr -d ' ')" ]; then
        echo -e "${GREEN}No active runs to cancel${NC}"
        return
    fi
    
    for run_id in $all_active; do
        if [ -n "$run_id" ]; then
            echo -n "Cancelling run $run_id... "
            if gh run cancel $run_id --repo $REPO >/dev/null 2>&1; then
                echo -e "${GREEN}✓${NC}"
            else
                echo -e "${RED}✗${NC}"
            fi
        fi
    done
    
    # Wait a bit for cancellations to process
    echo -e "${YELLOW}Waiting for cancellations to process...${NC}"
    sleep 5
}

# Delete all runs
delete_all_runs() {
    echo -e "${YELLOW}Deleting all runs...${NC}"
    
    local total_deleted=0
    local page=1
    
    while true; do
        # Get all runs (including different statuses)
        runs=$(gh api -X GET "/repos/$REPO/actions/runs?per_page=100&page=$page" --jq '.workflow_runs[].id')
        
        if [ -z "$runs" ]; then
            break
        fi
        
        for run_id in $runs; do
            echo -n "Deleting run $run_id... "
            
            # Try API deletion first
            if gh api -X DELETE "/repos/$REPO/actions/runs/$run_id" >/dev/null 2>&1; then
                echo -e "${GREEN}✓${NC}"
                ((total_deleted++))
            else
                # Try gh run delete
                if gh run delete $run_id --repo $REPO >/dev/null 2>&1; then
                    echo -e "${GREEN}✓ (gh)${NC}"
                    ((total_deleted++))
                else
                    echo -e "${RED}✗${NC}"
                fi
            fi
        done
        
        ((page++))
    done
    
    echo -e "${GREEN}Deleted $total_deleted runs${NC}"
}

# Main
echo -e "${BLUE}=== Force Delete All Workflow Runs ===${NC}"

# Get initial count
initial_count=$(gh api -X GET "/repos/$REPO/actions/runs" --jq '.total_count')
echo -e "${YELLOW}Total runs: $initial_count${NC}"

if [ "$initial_count" -eq 0 ]; then
    echo -e "${GREEN}No runs to delete${NC}"
    exit 0
fi

# Confirm
echo -e "${RED}This will cancel and delete ALL workflow runs!${NC}"
read -p "Continue? (yes/N) " -r
if [[ ! $REPLY == "yes" ]]; then
    echo "Cancelled"
    exit 1
fi

# Cancel active runs first
cancel_active_runs

# Delete all runs
delete_all_runs

# Final count
final_count=$(gh api -X GET "/repos/$REPO/actions/runs" --jq '.total_count')
echo -e "${BLUE}Remaining runs: $final_count${NC}"

if [ "$final_count" -gt 0 ]; then
    echo -e "${YELLOW}Some runs could not be deleted. Trying alternative method...${NC}"
    
    # Try using workflow-specific deletion
    workflows=$(gh workflow list --json id -q '.[].id')
    for workflow_id in $workflows; do
        echo "Processing workflow $workflow_id..."
        runs=$(gh run list --workflow $workflow_id --json databaseId -q '.[].databaseId' --limit 1000)
        for run_id in $runs; do
            gh run delete $run_id >/dev/null 2>&1 && echo -n "." || echo -n "x"
        done
        echo
    done
fi
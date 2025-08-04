#!/bin/bash

# Script to delete GitHub workflow runs
# Usage: ./delete-workflow-runs.sh [workflow_name] [--all]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to delete runs for a specific workflow
delete_workflow_runs() {
    local workflow_id=$1
    local workflow_name=$2
    
    echo -e "${YELLOW}Fetching runs for workflow: $workflow_name${NC}"
    
    # Get all run IDs for the workflow
    run_ids=$(gh run list --workflow "$workflow_id" --limit 1000 --json databaseId --jq '.[].databaseId')
    
    if [ -z "$run_ids" ]; then
        echo -e "${GREEN}No runs found for $workflow_name${NC}"
        return
    fi
    
    # Count runs
    run_count=$(echo "$run_ids" | wc -l)
    echo -e "${YELLOW}Found $run_count runs to delete${NC}"
    
    # Delete each run
    local deleted=0
    for run_id in $run_ids; do
        echo -n "Deleting run $run_id... "
        if gh run delete "$run_id" >/dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
            ((deleted++))
        else
            echo -e "${RED}✗${NC}"
        fi
    done
    
    echo -e "${GREEN}Deleted $deleted/$run_count runs for $workflow_name${NC}"
}

# Main script
main() {
    # Check if gh CLI is available
    if ! command -v gh &> /dev/null; then
        echo -e "${RED}Error: GitHub CLI (gh) is not installed${NC}"
        echo "Please install it from: https://cli.github.com/"
        exit 1
    fi
    
    # Check if authenticated
    if ! gh auth status &> /dev/null; then
        echo -e "${RED}Error: Not authenticated with GitHub${NC}"
        echo "Please run: gh auth login"
        exit 1
    fi
    
    # Parse arguments
    if [ "$1" == "--all" ]; then
        # Delete all workflow runs
        echo -e "${YELLOW}Deleting runs for ALL workflows...${NC}"
        echo -e "${RED}WARNING: This will delete all workflow run history!${NC}"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Cancelled"
            exit 0
        fi
        
        # Get all workflows
        while IFS=$'\t' read -r name id; do
            delete_workflow_runs "$id" "$name"
            echo
        done < <(gh workflow list --all | awk '{print $1"\t"$NF}')
        
    elif [ -n "$1" ]; then
        # Delete runs for specific workflow
        workflow_name="$1"
        
        # Find workflow ID
        workflow_id=$(gh workflow list --all | grep -i "^$workflow_name" | awk '{print $NF}')
        
        if [ -z "$workflow_id" ]; then
            echo -e "${RED}Error: Workflow '$workflow_name' not found${NC}"
            echo "Available workflows:"
            gh workflow list --all | awk '{print "  - " $1}'
            exit 1
        fi
        
        echo -e "${YELLOW}Deleting runs for workflow: $workflow_name${NC}"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Cancelled"
            exit 0
        fi
        
        delete_workflow_runs "$workflow_id" "$workflow_name"
        
    else
        # Show usage
        echo "Usage: $0 [workflow_name] [--all]"
        echo
        echo "Examples:"
        echo "  $0 CI                    # Delete all runs for CI workflow"
        echo "  $0 \"Security Audit\"      # Delete all runs for Security Audit workflow"
        echo "  $0 --all                 # Delete all runs for all workflows"
        echo
        echo "Available workflows:"
        gh workflow list --all | awk '{print "  - " $1}'
    fi
}

# Run main function
main "$@"
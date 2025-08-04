#!/bin/bash

# Script to delete ALL workflow runs from a GitHub repository
# This uses the GitHub API directly for more reliable deletion

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Get repository info
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)

if [ -z "$REPO" ]; then
    echo -e "${RED}Error: Could not determine repository${NC}"
    exit 1
fi

echo -e "${BLUE}Repository: $REPO${NC}"

# Function to delete runs in batches
delete_runs_batch() {
    local page=1
    local per_page=100
    local total_deleted=0
    
    while true; do
        echo -e "${YELLOW}Fetching runs (page $page)...${NC}"
        
        # Get runs for this page
        runs=$(gh api -X GET "/repos/$REPO/actions/runs?per_page=$per_page&page=$page" --jq '.workflow_runs[].id')
        
        if [ -z "$runs" ]; then
            echo -e "${GREEN}No more runs found${NC}"
            break
        fi
        
        # Count runs in this batch
        batch_count=$(echo "$runs" | wc -l | tr -d ' ')
        echo -e "${YELLOW}Found $batch_count runs in this batch${NC}"
        
        # Delete each run
        for run_id in $runs; do
            echo -n "Deleting run $run_id... "
            if gh api -X DELETE "/repos/$REPO/actions/runs/$run_id" >/dev/null 2>&1; then
                echo -e "${GREEN}✓${NC}"
                ((total_deleted++))
            else
                # Try using gh run delete as fallback
                if gh run delete $run_id --repo $REPO >/dev/null 2>&1; then
                    echo -e "${GREEN}✓ (via gh)${NC}"
                    ((total_deleted++))
                else
                    echo -e "${RED}✗${NC}"
                fi
            fi
            
            # Small delay to avoid rate limiting
            if [ $((total_deleted % 10)) -eq 0 ]; then
                sleep 0.5
            fi
        done
        
        # Check if we should continue to next page
        if [ $batch_count -lt $per_page ]; then
            break
        fi
        
        ((page++))
    done
    
    echo -e "${GREEN}Total deleted: $total_deleted runs${NC}"
}

# Function to get total run count
get_total_runs() {
    gh api -X GET "/repos/$REPO/actions/runs" --jq '.total_count'
}

# Main execution
main() {
    echo -e "${BLUE}=== GitHub Workflow Run Deletion ===${NC}"
    echo
    
    # Get initial count
    initial_count=$(get_total_runs)
    echo -e "${YELLOW}Current total runs: $initial_count${NC}"
    
    if [ "$initial_count" -eq 0 ]; then
        echo -e "${GREEN}No workflow runs to delete${NC}"
        exit 0
    fi
    
    # Confirm deletion
    echo -e "${RED}WARNING: This will delete ALL workflow runs!${NC}"
    echo -e "${RED}This action cannot be undone.${NC}"
    read -p "Are you sure you want to delete all $initial_count runs? (yes/N) " -r
    
    if [[ ! $REPLY == "yes" ]]; then
        echo "Cancelled"
        exit 0
    fi
    
    echo
    echo -e "${YELLOW}Starting deletion...${NC}"
    
    # Delete all runs
    delete_runs_batch
    
    # Get final count
    echo
    echo -e "${YELLOW}Verifying...${NC}"
    final_count=$(get_total_runs)
    echo -e "${BLUE}Remaining runs: $final_count${NC}"
    
    if [ "$final_count" -eq 0 ]; then
        echo -e "${GREEN}Successfully deleted all workflow runs!${NC}"
    else
        echo -e "${YELLOW}Note: $final_count runs remain. These might be protected or in progress.${NC}"
        echo "You can run this script again to retry deletion."
    fi
}

# Check prerequisites
if ! command -v gh &> /dev/null; then
    echo -e "${RED}Error: GitHub CLI (gh) is not installed${NC}"
    exit 1
fi

if ! gh auth status &> /dev/null; then
    echo -e "${RED}Error: Not authenticated with GitHub${NC}"
    echo "Please run: gh auth login"
    exit 1
fi

# Run main function
main
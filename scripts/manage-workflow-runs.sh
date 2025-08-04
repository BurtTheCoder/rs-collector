#!/bin/bash

# Enhanced script to manage GitHub workflow runs
# Supports filtering by status, date, and batch operations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
LIMIT=1000
DRY_RUN=false
FILTER_STATUS=""
FILTER_DAYS=""
KEEP_LATEST=0

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS] [WORKFLOW_NAME|--all]

Manage GitHub Actions workflow runs - delete, list, or analyze.

OPTIONS:
    -h, --help              Show this help message
    -d, --dry-run           Show what would be deleted without deleting
    -s, --status STATUS     Filter by status (failure, success, cancelled, etc.)
    -o, --older-than DAYS   Delete runs older than specified days
    -k, --keep-latest N     Keep the latest N runs (delete the rest)
    -l, --limit N           Maximum number of runs to process (default: 1000)
    --list                  List runs instead of deleting

EXAMPLES:
    # Delete all failed runs for CI workflow
    $0 --status failure CI

    # Delete all runs older than 30 days
    $0 --older-than 30 --all

    # Keep only the latest 10 runs for each workflow
    $0 --keep-latest 10 --all

    # Dry run - see what would be deleted
    $0 --dry-run --all

    # List all failed runs
    $0 --list --status failure --all

AVAILABLE WORKFLOWS:
EOF
    gh workflow list --all | awk '{print "    - " $1}'
}

# Function to parse date
days_ago() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        date -v-${1}d +%Y-%m-%d
    else
        # Linux
        date -d "$1 days ago" +%Y-%m-%d
    fi
}

# Function to delete or list runs
process_workflow_runs() {
    local workflow_id=$1
    local workflow_name=$2
    local action=$3  # "delete" or "list"
    
    echo -e "${BLUE}Processing workflow: $workflow_name${NC}"
    
    # Build query
    local query="gh run list --workflow \"$workflow_id\" --limit $LIMIT --json databaseId,status,conclusion,createdAt,event,headBranch"
    
    # Apply filters
    if [ -n "$FILTER_STATUS" ]; then
        query="$query --status $FILTER_STATUS"
    fi
    
    # Get runs
    local runs=$(eval "$query")
    
    # Apply date filter if specified
    if [ -n "$FILTER_DAYS" ]; then
        local cutoff_date=$(days_ago $FILTER_DAYS)
        runs=$(echo "$runs" | jq --arg date "$cutoff_date" '[.[] | select(.createdAt < $date)]')
    fi
    
    # Apply keep-latest filter if specified
    if [ $KEEP_LATEST -gt 0 ]; then
        local total_runs=$(echo "$runs" | jq 'length')
        if [ $total_runs -gt $KEEP_LATEST ]; then
            runs=$(echo "$runs" | jq --arg keep "$KEEP_LATEST" '[.[] | sort_by(.createdAt) | reverse][$keep|tonumber:]')
        else
            echo -e "${GREEN}Workflow has $total_runs runs, keeping all (less than $KEEP_LATEST)${NC}"
            return
        fi
    fi
    
    # Get run IDs
    local run_ids=$(echo "$runs" | jq -r '.[].databaseId')
    
    if [ -z "$run_ids" ]; then
        echo -e "${GREEN}No runs match the criteria${NC}"
        return
    fi
    
    # Count runs
    local run_count=$(echo "$run_ids" | wc -l | tr -d ' ')
    
    if [ "$action" == "list" ]; then
        echo -e "${YELLOW}Found $run_count matching runs:${NC}"
        echo "$runs" | jq -r '.[] | "\(.databaseId)\t\(.status)/\(.conclusion)\t\(.createdAt)\t\(.event)\t\(.headBranch)"' | \
            awk 'BEGIN {printf "%-12s %-20s %-25s %-10s %s\n", "Run ID", "Status", "Created", "Event", "Branch"} 
                 {printf "%-12s %-20s %-25s %-10s %s\n", $1, $2, $3, $4, $5}'
        return
    fi
    
    echo -e "${YELLOW}Found $run_count runs to delete${NC}"
    
    if [ "$DRY_RUN" == "true" ]; then
        echo -e "${BLUE}DRY RUN - Would delete these runs:${NC}"
        echo "$runs" | jq -r '.[] | "\(.databaseId)\t\(.status)/\(.conclusion)\t\(.createdAt)"' | head -20
        if [ $run_count -gt 20 ]; then
            echo "... and $((run_count - 20)) more"
        fi
        return
    fi
    
    # Confirm deletion
    read -p "Delete $run_count runs? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Skipped"
        return
    fi
    
    # Delete each run
    local deleted=0
    local failed=0
    for run_id in $run_ids; do
        echo -n "Deleting run $run_id... "
        if gh run delete "$run_id" >/dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
            ((deleted++))
        else
            echo -e "${RED}✗${NC}"
            ((failed++))
        fi
        
        # Add small delay to avoid rate limiting
        if [ $((deleted % 10)) -eq 0 ]; then
            sleep 0.5
        fi
    done
    
    echo -e "${GREEN}Deleted $deleted runs${NC}"
    if [ $failed -gt 0 ]; then
        echo -e "${RED}Failed to delete $failed runs${NC}"
    fi
}

# Main script
main() {
    local action="delete"
    local workflow_target=""
    
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
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -s|--status)
                FILTER_STATUS="$2"
                shift 2
                ;;
            -o|--older-than)
                FILTER_DAYS="$2"
                shift 2
                ;;
            -k|--keep-latest)
                KEEP_LATEST="$2"
                shift 2
                ;;
            -l|--limit)
                LIMIT="$2"
                shift 2
                ;;
            --list)
                action="list"
                shift
                ;;
            --all)
                workflow_target="all"
                shift
                ;;
            *)
                workflow_target="$1"
                shift
                ;;
        esac
    done
    
    # Validate workflow target
    if [ -z "$workflow_target" ]; then
        echo -e "${RED}Error: No workflow specified${NC}"
        usage
        exit 1
    fi
    
    # Show current filters
    echo -e "${BLUE}=== Workflow Run Management ===${NC}"
    if [ "$DRY_RUN" == "true" ]; then
        echo -e "${YELLOW}MODE: DRY RUN (no changes will be made)${NC}"
    fi
    if [ -n "$FILTER_STATUS" ]; then
        echo "Filter: Status = $FILTER_STATUS"
    fi
    if [ -n "$FILTER_DAYS" ]; then
        echo "Filter: Older than $FILTER_DAYS days"
    fi
    if [ $KEEP_LATEST -gt 0 ]; then
        echo "Filter: Keep latest $KEEP_LATEST runs"
    fi
    echo
    
    # Process workflows
    if [ "$workflow_target" == "all" ]; then
        # Process all workflows
        while IFS=$'\t' read -r name state id; do
            if [ "$state" == "active" ]; then
                process_workflow_runs "$id" "$name" "$action"
                echo
            fi
        done < <(gh workflow list --all | awk '{print $1"\t"$2"\t"$NF}')
    else
        # Process specific workflow
        # Find workflow ID (case-insensitive search)
        workflow_info=$(gh workflow list --all | grep -i "^$workflow_target" | head -1)
        
        if [ -z "$workflow_info" ]; then
            echo -e "${RED}Error: Workflow '$workflow_target' not found${NC}"
            echo "Available workflows:"
            gh workflow list --all | awk '{print "  - " $1}'
            exit 1
        fi
        
        workflow_id=$(echo "$workflow_info" | awk '{print $NF}')
        workflow_name=$(echo "$workflow_info" | awk '{print $1}')
        
        process_workflow_runs "$workflow_id" "$workflow_name" "$action"
    fi
    
    echo -e "${GREEN}Done!${NC}"
}

# Run main function
main "$@"
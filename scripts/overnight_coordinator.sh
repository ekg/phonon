#!/bin/bash
# Phonon Overnight Agent Swarm Coordinator
# Continuously monitors workgraph and spawns agents for newly ready tasks

LOG_FILE=".workgraph/coordinator.log"
EXECUTOR="claude"
TIMEOUT="2h"
CHECK_INTERVAL=60  # seconds between checks

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

cleanup_dead_agents() {
    log "Checking for dead agents..."
    wg dead-agents --cleanup --threshold 5 2>/dev/null || true
}

spawn_ready_tasks() {
    local spawned=0

    # Get list of ready tasks
    READY=$(wg ready --json 2>/dev/null | jq -r '.[].id' 2>/dev/null)

    if [ -z "$READY" ]; then
        return 0
    fi

    for task_id in $READY; do
        log "Spawning agent for: $task_id"
        if wg spawn "$task_id" --executor "$EXECUTOR" --timeout "$TIMEOUT" 2>&1 | tee -a "$LOG_FILE"; then
            spawned=$((spawned + 1))
            sleep 2  # Brief pause between spawns
        fi
    done

    echo $spawned
}

show_status() {
    log "=== Status Report ==="
    wg list 2>/dev/null | grep -E "^\[" | head -20
    echo ""
    log "Active agents: $(wg agents --alive 2>/dev/null | tail -n1)"
    log "Ready tasks: $(wg ready 2>/dev/null | wc -l)"
    log "Blocked tasks: $(wg list 2>/dev/null | grep -c blocked || echo 0)"

    # Show completion progress
    TOTAL=$(wg list 2>/dev/null | wc -l)
    DONE=$(wg list 2>/dev/null | grep -c "^\[x\]" || echo 0)
    log "Progress: $DONE / $TOTAL tasks complete"
}

main() {
    log "=== Phonon Overnight Coordinator Starting ==="
    log "Executor: $EXECUTOR"
    log "Timeout: $TIMEOUT"
    log "Check interval: ${CHECK_INTERVAL}s"

    while true; do
        # Clean up any dead agents
        cleanup_dead_agents

        # Spawn agents for any newly ready tasks
        spawned=$(spawn_ready_tasks)

        if [ "$spawned" -gt 0 ]; then
            log "Spawned $spawned new agent(s)"
        fi

        # Show status every 5 minutes
        if [ $((SECONDS % 300)) -lt $CHECK_INTERVAL ]; then
            show_status
        fi

        # Check if we're done
        REMAINING=$(wg list 2>/dev/null | grep -c "^\[ \]" || echo 0)
        IN_PROGRESS=$(wg list 2>/dev/null | grep -c "^\[>\]" || echo 0)

        if [ "$REMAINING" -eq 0 ] && [ "$IN_PROGRESS" -eq 0 ]; then
            log "=== ALL TASKS COMPLETE ==="
            show_status
            break
        fi

        sleep $CHECK_INTERVAL
    done
}

# Run if called directly
if [ "${BASH_SOURCE[0]}" == "${0}" ]; then
    main "$@"
fi

#!/usr/bin/env bash
CMD_FILE="/tmp/atelier-cmd"
SESSION_NAME="atelier"
REPL_PANE="${SESSION_NAME}:0.2"

# Ensure the command file exists
touch "$CMD_FILE"

LAST_MTIME=0

while true; do
  # Check if the tmux session is still running
  if ! tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
    # Clean up temp files and exit
    rm -f "$CMD_FILE"
    rm -f "/tmp/atelier-vars.csv"
    exit 0
  fi

  if [ -f "$CMD_FILE" ]; then
    CURRENT_MTIME=$(stat -c %Y "$CMD_FILE" 2>/dev/null || echo 0)
    if [ "$CURRENT_MTIME" -ne "$LAST_MTIME" ]; then
      LAST_MTIME=$CURRENT_MTIME
      
      # Wait a tiny bit to make sure writing is complete
      sleep 0.05
      
      if [ -s "$CMD_FILE" ]; then
        # Load code into tmux buffer and paste to REPL pane
        tmux load-buffer -b atelier_buf "$CMD_FILE" 2>/dev/null
        tmux paste-buffer -b atelier_buf -t "$REPL_PANE"
        tmux send-keys -t "$REPL_PANE" Enter
      fi
    fi
  fi
  sleep 0.1
done

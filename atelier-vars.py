#!/usr/bin/env python3
import os
import sys
import csv
import time

CSV_PATH = "/tmp/atelier-vars.csv"

# ANSI color codes
RESET = "\033[0m"
BOLD = "\033[1m"
HEADER_COLOR = "\033[1;36m"  # Cyan Bold
BORDER_COLOR = "\033[38;5;242m"  # Dark Grey
NAME_COLOR = "\033[1;37m"  # White Bold
TYPE_COLOR = "\033[33m"  # Yellow
VAL_COLOR_INT = "\033[32m"  # Green
VAL_COLOR_STR = "\033[35m"  # Magenta
VAL_COLOR_DEFAULT = "\033[37m"  # White

def clear_screen():
    sys.stdout.write("\033[H\033[2J")
    sys.stdout.flush()

def format_value(val, var_type):
    # Truncate value if it's too long
    if len(val) > 40:
        val = val[:37] + "..."
    if var_type in ("Int", "Float"):
        return f"{VAL_COLOR_INT}{val}{RESET}"
    elif var_type == "String":
        return f"{VAL_COLOR_STR}\"{val}\"{RESET}"
    else:
        return f"{VAL_COLOR_DEFAULT}{val}{RESET}"

def draw_table(rows):
    if not rows:
        print(f"\n {BOLD}Atelier Variable Watcher{RESET}")
        print(f" {BORDER_COLOR}─────────────────────────{RESET}")
        print(" No user variables defined yet.")
        print("\n Run some code in the REPL (e.g. `x = 42`)")
        print(" to see variables here.")
        return

    # Calculate column widths
    col_widths = {
        "name": max(len(r["name"]) for r in rows),
        "type": max(len(r["type"]) for r in rows),
        "value": max(len(r["value"]) for r in rows)
    }
    # Ensure minimum widths for headers
    col_widths["name"] = max(col_widths["name"], 4)
    col_widths["type"] = max(col_widths["type"], 4)
    col_widths["value"] = max(col_widths["value"], 5)

    # Box-drawing characters
    bc = BORDER_COLOR
    rst = RESET
    
    # Top border
    print(f"{bc}┌{'─' * (col_widths['name']+2)}┬{'─' * (col_widths['type']+2)}┬{'─' * (col_widths['value']+2)}┐{rst}")
    # Header row
    name_hdr = f"{HEADER_COLOR}Name{rst}".ljust(col_widths['name'] + len(HEADER_COLOR) + len(rst))
    type_hdr = f"{HEADER_COLOR}Type{rst}".ljust(col_widths['type'] + len(HEADER_COLOR) + len(rst))
    val_hdr = f"{HEADER_COLOR}Value{rst}".ljust(col_widths['value'] + len(HEADER_COLOR) + len(rst))
    print(f"{bc}│{rst} {name_hdr} {bc}│{rst} {type_hdr} {bc}│{rst} {val_hdr} {bc}│{rst}")
    # Separator
    print(f"{bc}├{'─' * (col_widths['name']+2)}┼{'─' * (col_widths['type']+2)}┼{'─' * (col_widths['value']+2)}┤{rst}")
    
    # Rows
    for r in rows:
        name_cell = f"{NAME_COLOR}{r['name']}{rst}".ljust(col_widths['name'] + len(NAME_COLOR) + len(rst))
        type_cell = f"{TYPE_COLOR}{r['type']}{rst}".ljust(col_widths['type'] + len(TYPE_COLOR) + len(rst))
        
        # Color value depending on type
        v_formatted = format_value(r['value'], r['type'])
        val_cell = v_formatted + " " * (col_widths['value'] - len(r['value']))
        
        print(f"{bc}│{rst} {name_cell} {bc}│{rst} {type_cell} {bc}│{rst} {val_cell} {bc}│{rst}")

    # Bottom border
    print(f"{bc}└{'─' * (col_widths['name']+2)}┴{'─' * (col_widths['type']+2)}┴{'─' * (col_widths['value']+2)}┘{rst}")

def main():
    last_mtime = 0
    last_content = None

    while True:
        try:
            if os.path.exists(CSV_PATH):
                mtime = os.path.getmtime(CSV_PATH)
                if mtime != last_mtime:
                    last_mtime = mtime
                    rows = []
                    time.sleep(0.02)
                    with open(CSV_PATH, "r", newline="", encoding="utf-8") as f:
                        reader = csv.DictReader(f)
                        for r in reader:
                            if "name" in r and "type" in r and "value" in r:
                                rows.append(r)
                    
                    if rows != last_content:
                        last_content = rows
                        clear_screen()
                        draw_table(rows)
            else:
                if last_content is not None or last_mtime == 0:
                    last_content = None
                    last_mtime = 0
                    clear_screen()
                    draw_table([])
        except Exception as e:
            pass
        time.sleep(0.1)

if __name__ == "__main__":
    main()

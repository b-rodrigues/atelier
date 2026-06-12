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
NAME_COLOR = "\033[1m"  # Bold default foreground
TYPE_COLOR = "\033[33m"  # Yellow
VAL_COLOR_INT = "\033[32m"  # Green
VAL_COLOR_STR = "\033[35m"  # Magenta
VAL_COLOR_DEFAULT = "\033[0m"  # Default reset/foreground

def clear_screen():
    sys.stdout.write("\033[H\033[2J")
    sys.stdout.flush()

def get_display_val(val, var_type):
    # Truncate value if it's too long
    if len(val) > 40:
        val = val[:37] + "..."
    if var_type == "String":
        return f'"{val}"'
    return val

def get_color_for_type(var_type):
    if var_type in ("Int", "Float"):
        return VAL_COLOR_INT
    elif var_type == "String":
        return VAL_COLOR_STR
    else:
        return VAL_COLOR_DEFAULT

def draw_table(rows):
    if not rows:
        print(f"\n {BOLD}Atelier Variable Watcher{RESET}")
        print(f" {BORDER_COLOR}─────────────────────────{RESET}")
        print(" No user variables defined yet.")
        print("\n Run some code in the REPL (e.g. `x = 42`)")
        print(" to see variables here.")
        return

    rows_display = []
    for r in rows:
        disp_val = get_display_val(r["value"], r["type"])
        rows_display.append({
            "name": r["name"],
            "type": r["type"],
            "display_val": disp_val,
            "color": get_color_for_type(r["type"])
        })

    # Calculate column widths
    col_widths = {
        "name": max(len(r["name"]) for r in rows_display),
        "type": max(len(r["type"]) for r in rows_display),
        "value": max(len(r["display_val"]) for r in rows_display)
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
    for r in rows_display:
        name_cell = f"{NAME_COLOR}{r['name']}{rst}".ljust(col_widths['name'] + len(NAME_COLOR) + len(rst))
        type_cell = f"{TYPE_COLOR}{r['type']}{rst}".ljust(col_widths['type'] + len(TYPE_COLOR) + len(rst))
        
        # Color value depending on type
        v_formatted = f"{r['color']}{r['display_val']}{rst}"
        val_cell = v_formatted + " " * (col_widths['value'] - len(r['display_val']))
        
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
            clear_screen()
            print(f"Error in variable watcher: {e}")
        time.sleep(0.1)

if __name__ == "__main__":
    main()

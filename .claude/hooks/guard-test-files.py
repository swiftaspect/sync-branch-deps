#!/usr/bin/env python3
# === Copied from copier-template-typescript ===
# Source: template/.claude/hooks/guard-test-files.py at commit
# e351500d435b8f8e298eaf746118d3d865f7a9dc. This repo owns this copy; a
# change here does not need a template change.
#
# PreToolUse hook for existing test files on a fix/ branch. See CONTRIBUTING.md.
import json
import os
import re
import subprocess
import sys

EDIT_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}
TEST_RE = re.compile(r"(^|/)(test|tests|__tests__)/|\.(test|spec)\.[cm]?[jt]sx?$")
WRITE_RE = re.compile(r"(sed\s+-i|perl\s+-i|\btee\b|>{1,2}\s*|\bcp\b|\bmv\b|\brm\b|\btruncate\b|\bpatch\b)")
PATH_RE = re.compile(r"[\w./-]+")


def project_dir(payload):
    return os.environ.get("CLAUDE_PROJECT_DIR") or payload.get("cwd") or os.getcwd()


def branch(root):
    try:
        out = subprocess.run(
            ["git", "-C", root, "symbolic-ref", "--short", "HEAD"],
            capture_output=True, text=True, timeout=5, check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return ""
    return out.stdout.strip()


def is_existing_test(root, path):
    rel = os.path.relpath(os.path.abspath(os.path.join(root, path)), root)
    return bool(TEST_RE.search(rel)) and os.path.isfile(os.path.join(root, rel))


def ask(reason):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "ask",
        "permissionDecisionReason": reason,
    }}))
    sys.exit(0)


def main():
    payload = json.load(sys.stdin)
    root = project_dir(payload)
    if not branch(root).startswith("fix/"):
        return
    tool = payload.get("tool_name", "")
    tool_input = payload.get("tool_input", {})
    reason = (
        "{path} is an existing test on a fix branch. A test is never changed to make the code "
        "pass. Confirm only when the owner agreed the test itself is wrong."
    )
    if tool in EDIT_TOOLS:
        target = tool_input.get("file_path") or tool_input.get("notebook_path") or ""
        if target and is_existing_test(root, target):
            ask(reason.format(path=os.path.relpath(os.path.abspath(target), root)))
    elif tool == "Bash":
        command = tool_input.get("command", "")
        if not WRITE_RE.search(command):
            return
        for token in PATH_RE.findall(command):
            if is_existing_test(root, token):
                ask(reason.format(path=token))
                return


if __name__ == "__main__":
    main()

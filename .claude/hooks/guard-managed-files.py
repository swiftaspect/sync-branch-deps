#!/usr/bin/env python3
# === Copied from copier-template-typescript ===
# Source: template/.claude/hooks/guard-managed-files.py at commit
# e351500d435b8f8e298eaf746118d3d865f7a9dc. This repo owns this copy; a
# change here does not need a template change.
#
# PreToolUse hook for paths listed in .copier-managed-files. See CONTRIBUTING.md.
import json
import os
import re
import sys

EDIT_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}
# Shell forms that write a file in place. The match is a heuristic; a false
# positive costs one rephrased command, a miss costs a reverted commit.
WRITE_RE = re.compile(r"(sed\s+-i|perl\s+-i|\btee\b|>{1,2}\s*|\bcp\b|\bmv\b|\brm\b|\btruncate\b|\bpatch\b)")


def project_dir(payload):
    return os.environ.get("CLAUDE_PROJECT_DIR") or payload.get("cwd") or os.getcwd()


def managed_paths(root):
    manifest = os.path.join(root, ".copier-managed-files")
    try:
        with open(manifest, encoding="utf-8") as handle:
            lines = handle.read().splitlines()
    except OSError:
        return set()
    return {line.strip() for line in lines if line.strip() and not line.startswith("#")}


def relative(root, path):
    return os.path.relpath(os.path.abspath(os.path.join(root, path)), root)


def deny(reason):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": reason,
    }}))
    sys.exit(0)


def main():
    payload = json.load(sys.stdin)
    root = project_dir(payload)
    managed = managed_paths(root)
    if not managed:
        return
    tool = payload.get("tool_name", "")
    tool_input = payload.get("tool_input", {})
    reason = (
        "{path} is managed by copier-template-typescript. Open the template change as its own "
        "pull request first, then put `Blocked by copier-template-typescript#<n>` at the top of "
        "this pull request body."
    )
    if tool in EDIT_TOOLS:
        target = tool_input.get("file_path") or tool_input.get("notebook_path") or ""
        if target and relative(root, target) in managed:
            deny(reason.format(path=relative(root, target)))
    elif tool == "Bash":
        command = tool_input.get("command", "")
        if not WRITE_RE.search(command):
            return
        for path in sorted(managed, key=len, reverse=True):
            if re.search(r"(^|[\s/\"'=])" + re.escape(path) + r"($|[\s\"';)])", command):
                deny(reason.format(path=path))


if __name__ == "__main__":
    main()

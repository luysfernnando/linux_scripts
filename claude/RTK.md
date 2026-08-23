# RTK - Rust Token Killer

Token-optimized CLI proxy — cuts up to 90% of bash output. All Bash commands auto-rewritten by hook (`git status` → `rtk git status`, transparent, 0 extra tokens).

## Meta commands (call directly, hook doesn't rewrite these)

```bash
rtk gain              # token savings analytics
rtk gain --history    # usage history + savings
rtk discover          # find missed optimization opportunities
rtk proxy <cmd>       # raw output, no filtering (debug)
```

Broken (`rtk gain` fails / unknown command)? `which rtk` — reachingforthejack/rtk (Rust Type Kit) name-collides. Verify with `rtk --version`.

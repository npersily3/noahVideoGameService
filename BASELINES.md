``# Phase 4 Baselines

Captured 2026-07-04, local run, JSON wire protocol, 300 bots via `client/launch_bots.ps1 -Bots 300` (~20s).

Note: bots sleep 3-8ms between sends (~180Hz), but the real client (`client/main.go`,
`INPUT_FREQUENCY = 60`) sends at a fixed 16.67ms (60Hz). Bots are ~3x faster than any real
client, so recv-side numbers here are a stress test, not representative load. Re-run with
bot sleep ~15-18ms for an apples-to-apples baseline.

## Idle (1 client)
- tick max: 0.04-0.17ms, 0 overruns/s

## Under load (300 clients, bots at 3-8ms/send)
- sent: ~430,000-440,000 KB/s (~430 MB/s)
- recv: ~7,800 KB/s (~7.8 MB/s)
- per-client: ~1,470 KB/s
- tick max: 2.8-8.5ms (budget 16.67ms), 0 overruns/s

## Notes
- Sent bandwidth is O(clients x world_size): full snapshot broadcast to every client every tick,
  not per-client delta. Dominant lever is Phase 5 items 16/17 (delta + relevance filtering),
  not just switching JSON -> binary.
- Tick loop has headroom at 300 clients (max 8.5ms vs 16.67ms budget) — CPU is not yet the
  bottleneck, bandwidth is.

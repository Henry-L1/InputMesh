# InputMesh patch

This directory vendors the MIT-licensed `rdev` 0.5.3 crate.

InputMesh changes macOS event conversion so it does not call the TIS/TSM
keyboard-layout APIs from the CoreGraphics event-tap thread. macOS 15 aborts
that call with `_dispatch_assert_queue_fail`. InputMesh transports physical key
codes and does not use `Event::name`, so leaving that optional field empty is
the correct behaviour for this application.

The patch also exposes native macOS pointer deltas, includes dragged mouse
events, and tags rdev-generated CoreGraphics events with a fixed user-data
marker. The grab callback uses that marker to prevent injected remote input
from being captured and sent back as new physical input.

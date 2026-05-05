"""test_parser — prototype of restructured ability parser.

This package demonstrates a priority-based rule registry approach that:
- Builds dispatch/condition/effect tables ONCE at module import (not per-call)
- Uses explicit priority integers instead of implicit list position
- Logs unmatched patterns for easy gap detection
- Supports per-rule unit testing
- Allows rule injection at any priority level
"""

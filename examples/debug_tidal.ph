-- Debug Tidal sample patterns

-- Test individual samples first
~test_bd = s "bd"
~test_sn = s "sn"
~test_hh = s "hh"

-- Mix them
out = ~test_bd + ~test_sn + ~test_hh # mul 0.3
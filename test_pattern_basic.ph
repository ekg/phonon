-- Test what patterns actually work

-- Test 1: Explicit numbers
gate1 = "1 0 1 0"
test1 = sine 220 * gate1

-- Test 2: Euclidean notation (might not work?)
gate2 = "1(4,16)"
test2 = sine 330 * gate2

out test1 * 0.3

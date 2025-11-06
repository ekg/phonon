tempo: 2.0
-- Test voice pool shrinking:
-- 1. First cycle triggers 100 voices (grows to ~108)
-- 2. Subsequent cycles use only 4 voices
-- 3. Pool should shrink back down after voices finish
out: s "bd*100 ~ ~ ~ bd*4 bd*4 bd*4 bd*4"

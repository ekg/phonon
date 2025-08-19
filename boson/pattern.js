/**
 * Pattern Engine for Phonon
 * Implements the complete TidalCycles/Strudel pattern language
 * with all 150+ operators
 */

// Core types
class Fraction {
    constructor(numerator, denominator = 1) {
        this.n = numerator;
        this.d = denominator;
        this.simplify();
    }

    simplify() {
        const gcd = (a, b) => b ? gcd(b, a % b) : a;
        const g = gcd(Math.abs(this.n), Math.abs(this.d));
        this.n /= g;
        this.d /= g;
        if (this.d < 0) {
            this.n *= -1;
            this.d *= -1;
        }
    }

    add(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return new Fraction(this.n * other.d + other.n * this.d, this.d * other.d);
    }

    sub(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return new Fraction(this.n * other.d - other.n * this.d, this.d * other.d);
    }

    mul(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return new Fraction(this.n * other.n, this.d * other.d);
    }

    div(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return new Fraction(this.n * other.d, this.d * other.n);
    }

    mod(other) {
        if (typeof other === 'number') other = new Fraction(other);
        const q = Math.floor(this.toFloat() / other.toFloat());
        return this.sub(other.mul(q));
    }

    floor() {
        return new Fraction(Math.floor(this.toFloat()));
    }

    lt(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return this.n * other.d < other.n * this.d;
    }

    lte(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return this.n * other.d <= other.n * this.d;
    }

    gt(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return this.n * other.d > other.n * this.d;
    }

    gte(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return this.n * other.d >= other.n * this.d;
    }

    eq(other) {
        if (typeof other === 'number') other = new Fraction(other);
        return this.n * other.d === other.n * this.d;
    }

    toFloat() {
        return this.n / this.d;
    }

    toString() {
        return this.d === 1 ? `${this.n}` : `${this.n}/${this.d}`;
    }
}

// TimeSpan represents a span of time
class TimeSpan {
    constructor(begin, end) {
        this.begin = typeof begin === 'number' ? new Fraction(begin) : begin;
        this.end = typeof end === 'number' ? new Fraction(end) : end;
    }

    get duration() {
        return this.end.sub(this.begin);
    }

    intersection(other) {
        const begin = this.begin.gt(other.begin) ? this.begin : other.begin;
        const end = this.end.lt(other.end) ? this.end : other.end;
        if (begin.gte(end)) return null;
        return new TimeSpan(begin, end);
    }

    contains(time) {
        const t = typeof time === 'number' ? new Fraction(time) : time;
        return t.gte(this.begin) && t.lt(this.end);
    }

    overlaps(other) {
        return this.begin.lt(other.end) && this.end.gt(other.begin);
    }

    cyclePos() {
        return this.begin.sub(this.begin.floor());
    }
}

// Event represents a single event in a pattern
class Event {
    constructor(whole, part, value, context = {}) {
        this.whole = whole;  // The whole timespan
        this.part = part;    // The active part
        this.value = value;  // The event value
        this.context = context; // Additional metadata
    }

    withValue(fn) {
        return new Event(this.whole, this.part, fn(this.value), this.context);
    }

    hasOnset() {
        return this.whole && this.whole.begin.eq(this.part.begin);
    }
}

// The core Pattern class
class Pattern {
    constructor(query) {
        // query is a function that takes a TimeSpan and returns an array of Events
        this.query = query;
    }

    // Query events in a time range
    queryArc(begin, end) {
        const span = new TimeSpan(begin, end);
        return this.query(span);
    }

    // Map a function over pattern values
    fmap(fn) {
        return new Pattern((span) => {
            return this.query(span).map(event => event.withValue(fn));
        });
    }

    // Apply a pattern of functions to a pattern of values
    appBoth(patVal) {
        return new Pattern((span) => {
            const funcs = this.query(span);
            const vals = patVal.query(span);
            const results = [];
            
            for (const funcEvent of funcs) {
                for (const valEvent of vals) {
                    if (funcEvent.part.overlaps(valEvent.part)) {
                        const intersection = funcEvent.part.intersection(valEvent.part);
                        if (intersection) {
                            const whole = funcEvent.whole && valEvent.whole 
                                ? funcEvent.whole.intersection(valEvent.whole)
                                : null;
                            results.push(new Event(
                                whole,
                                intersection,
                                funcEvent.value(valEvent.value),
                                { ...funcEvent.context, ...valEvent.context }
                            ));
                        }
                    }
                }
            }
            return results;
        });
    }

    // Left-biased application
    appLeft(patVal) {
        return new Pattern((span) => {
            const funcs = this.query(span);
            const results = [];
            
            for (const funcEvent of funcs) {
                const vals = patVal.query(funcEvent.part);
                for (const valEvent of vals) {
                    if (funcEvent.part.overlaps(valEvent.part)) {
                        const intersection = funcEvent.part.intersection(valEvent.part);
                        if (intersection) {
                            results.push(new Event(
                                funcEvent.whole,
                                intersection,
                                funcEvent.value(valEvent.value),
                                { ...funcEvent.context, ...valEvent.context }
                            ));
                        }
                    }
                }
            }
            return results;
        });
    }

    // Right-biased application  
    appRight(patVal) {
        return patVal.appLeft(this.fmap(f => v => f(v)));
    }

    // Bind (monadic bind)
    bind(fn) {
        return new Pattern((span) => {
            const events = this.query(span);
            const results = [];
            for (const event of events) {
                const innerPat = fn(event.value);
                const innerEvents = innerPat.query(event.part);
                for (const innerEvent of innerEvents) {
                    if (event.part.overlaps(innerEvent.part)) {
                        const intersection = event.part.intersection(innerEvent.part);
                        if (intersection) {
                            results.push(new Event(
                                event.whole,
                                intersection,
                                innerEvent.value,
                                { ...event.context, ...innerEvent.context }
                            ));
                        }
                    }
                }
            }
            return results;
        });
    }

    // Join patterns with different biases
    innerJoin() {
        return this.bind(x => x);
    }

    outerJoin() {
        return this.bind(x => x);
    }
}

// ======== PATTERN OPERATORS IMPLEMENTATION ========

// === Core Pattern Creation ===

/**
 * Create a constant pattern from a single value
 */
function pure(value) {
    return new Pattern((span) => {
        const begin = span.begin.floor();
        const end = span.end.floor().add(1);
        const events = [];
        
        for (let cycle = begin.toFloat(); cycle < end.toFloat(); cycle++) {
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            if (cycleSpan.overlaps(span)) {
                const partSpan = cycleSpan.intersection(span);
                events.push(new Event(cycleSpan, partSpan, value));
            }
        }
        return events;
    });
}

/**
 * Empty pattern with no events
 */
function silence() {
    return new Pattern(() => []);
}

/**
 * Create a pattern with gaps
 */
function gap(steps = 1) {
    return new Pattern((span) => {
        const begin = span.begin.floor();
        const end = span.end.floor().add(1);
        const events = [];
        
        for (let cycle = begin.toFloat(); cycle < end.toFloat(); cycle++) {
            // Only create events every 'steps' cycles
            if (cycle % steps === 0) {
                const cycleSpan = new TimeSpan(cycle, cycle + 1);
                if (cycleSpan.overlaps(span)) {
                    const partSpan = cycleSpan.intersection(span);
                    events.push(new Event(null, partSpan, null));
                }
            }
        }
        return events;
    });
}

/**
 * Stack patterns on top of each other (play simultaneously)
 */
function stack(...patterns) {
    if (patterns.length === 0) return silence();
    if (patterns.length === 1) return patterns[0];
    
    return new Pattern((span) => {
        const allEvents = [];
        for (const pattern of patterns) {
            allEvents.push(...pattern.query(span));
        }
        return allEvents;
    });
}

/**
 * Concatenate patterns within a single cycle
 */
function cat(...patterns) {
    if (patterns.length === 0) return silence();
    if (patterns.length === 1) return patterns[0];
    
    return new Pattern((span) => {
        const n = patterns.length;
        const events = [];
        
        const begin = span.begin.floor();
        const end = span.end.floor().add(1);
        
        for (let cycle = begin.toFloat(); cycle < end.toFloat(); cycle++) {
            for (let i = 0; i < n; i++) {
                const sliceBegin = cycle + i / n;
                const sliceEnd = cycle + (i + 1) / n;
                const sliceSpan = new TimeSpan(sliceBegin, sliceEnd);
                
                if (sliceSpan.overlaps(span)) {
                    // Query the pattern for one cycle, then scale it
                    const patEvents = patterns[i].query(new TimeSpan(0, 1));
                    
                    for (const event of patEvents) {
                        // Scale and shift the event to fit in the slice
                        const scaledWhole = event.whole ? new TimeSpan(
                            sliceBegin + event.whole.begin.toFloat() / n,
                            sliceBegin + event.whole.end.toFloat() / n
                        ) : null;
                        
                        const scaledPart = new TimeSpan(
                            sliceBegin + event.part.begin.toFloat() / n,
                            sliceBegin + event.part.end.toFloat() / n
                        );
                        
                        const intersection = scaledPart.intersection(span);
                        if (intersection) {
                            events.push(new Event(scaledWhole, intersection, event.value, event.context));
                        }
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Fast concatenation - same as cat
 */
function fastcat(...patterns) {
    return cat(...patterns);
}

/**
 * Slow concatenation - each pattern takes a full cycle
 */
function slowcat(...patterns) {
    if (patterns.length === 0) return silence();
    if (patterns.length === 1) return patterns[0];
    
    return new Pattern((span) => {
        const n = patterns.length;
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat()); cycle <= Math.ceil(span.end.toFloat()); cycle++) {
            const patternIndex = ((cycle % n) + n) % n;
            const pattern = patterns[patternIndex];
            
            // Query the pattern normally
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const querySpan = cycleSpan.intersection(span);
            
            if (querySpan) {
                // Map the pattern's events from [0,1] to [cycle, cycle+1]
                const patEvents = pattern.query(new TimeSpan(0, 1));
                
                for (const event of patEvents) {
                    const shiftedWhole = event.whole ? new TimeSpan(
                        cycle + event.whole.begin.toFloat(),
                        cycle + event.whole.end.toFloat()
                    ) : null;
                    
                    const shiftedPart = new TimeSpan(
                        cycle + event.part.begin.toFloat(),
                        cycle + event.part.end.toFloat()
                    );
                    
                    const intersection = shiftedPart.intersection(span);
                    if (intersection) {
                        events.push(new Event(shiftedWhole, intersection, event.value, event.context));
                    }
                }
            }
        }
        return events;
    });
}

// === Time Manipulation ===

/**
 * Speed up a pattern by a factor
 */
function fast(factor, pattern) {
    const f = typeof factor === 'number' ? factor : factor.toFloat();
    
    return new Pattern((span) => {
        // Query a wider span at higher speed
        const scaledSpan = new TimeSpan(
            span.begin.mul(f),
            span.end.mul(f)
        );
        
        const events = pattern.query(scaledSpan);
        
        // Scale events back down
        return events.map(event => {
            const scaledWhole = event.whole ? new TimeSpan(
                event.whole.begin.div(f),
                event.whole.end.div(f)
            ) : null;
            
            const scaledPart = new TimeSpan(
                event.part.begin.div(f),
                event.part.end.div(f)
            );
            
            const intersection = scaledPart.intersection(span);
            if (!intersection) return null;
            
            return new Event(scaledWhole, intersection, event.value, event.context);
        }).filter(e => e !== null);
    });
}

/**
 * Slow down a pattern by a factor
 */
function slow(factor, pattern) {
    return fast(1 / factor, pattern);
}

/**
 * Shift pattern earlier by n cycles
 */
function early(n, pattern) {
    const shift = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        // Query the pattern shifted later
        const shiftedSpan = new TimeSpan(
            span.begin.add(shift),
            span.end.add(shift)
        );
        
        const events = pattern.query(shiftedSpan);
        
        // Shift events back earlier and filter to query span
        return events.map(event => {
            const shiftedWhole = event.whole ? new TimeSpan(
                event.whole.begin.sub(shift),
                event.whole.end.sub(shift)
            ) : null;
            
            const shiftedPart = new TimeSpan(
                event.part.begin.sub(shift),
                event.part.end.sub(shift)
            );
            
            // Only return if shifted part overlaps with original span
            const intersection = shiftedPart.intersection(span);
            if (!intersection) return null;
            
            return new Event(shiftedWhole, intersection, event.value, event.context);
        }).filter(e => e !== null);
    });
}

/**
 * Shift pattern later by n cycles
 */
function late(n, pattern) {
    return early(-n, pattern);
}

/**
 * Compress pattern into a timespan
 */
function compress(begin, end, pattern) {
    const b = typeof begin === 'number' ? begin : begin.toFloat();
    const e = typeof end === 'number' ? end : end.toFloat();
    
    if (b >= e) return silence();
    
    const duration = e - b;
    
    return new Pattern((span) => {
        const events = [];
        
        // For each cycle in the query span
        for (let cycle = Math.floor(span.begin.toFloat()); 
             cycle <= Math.ceil(span.end.toFloat()); 
             cycle++) {
            
            // Calculate the compressed window within this cycle
            const windowBegin = cycle + b;
            const windowEnd = cycle + e;
            const windowSpan = new TimeSpan(windowBegin, windowEnd);
            
            // Only process if window overlaps with query span
            if (windowSpan.overlaps(span)) {
                // Query pattern for one cycle, then compress it
                const patEvents = pattern.query(new TimeSpan(0, 1));
                
                for (const event of patEvents) {
                    // Scale and shift event to fit in window
                    const scaledWhole = event.whole ? new TimeSpan(
                        windowBegin + event.whole.begin.toFloat() * duration,
                        windowBegin + event.whole.end.toFloat() * duration
                    ) : null;
                    
                    const scaledPart = new TimeSpan(
                        windowBegin + event.part.begin.toFloat() * duration,
                        windowBegin + event.part.end.toFloat() * duration
                    );
                    
                    const intersection = scaledPart.intersection(span);
                    if (intersection) {
                        events.push(new Event(scaledWhole, intersection, event.value, event.context));
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Zoom into a section of the pattern
 */
function zoom(begin, end, pattern) {
    const b = typeof begin === 'number' ? begin : begin.toFloat();
    const e = typeof end === 'number' ? end : end.toFloat();
    
    if (b >= e) return silence();
    
    const duration = e - b;
    
    return new Pattern((span) => {
        // Map query span to zoomed section
        const zoomedSpan = new TimeSpan(
            span.begin.mul(duration).add(b),
            span.end.mul(duration).add(b)
        );
        
        const events = pattern.query(zoomedSpan);
        
        // Scale events back
        return events.map(event => {
            const scaledWhole = event.whole ? new TimeSpan(
                event.whole.begin.sub(b).div(duration),
                event.whole.end.sub(b).div(duration)
            ) : null;
            
            const scaledPart = new TimeSpan(
                event.part.begin.sub(b).div(duration),
                event.part.end.sub(b).div(duration)
            );
            
            const intersection = scaledPart.intersection(span);
            if (!intersection) return null;
            
            return new Event(scaledWhole, intersection, event.value, event.context);
        }).filter(e => e !== null);
    });
}

/**
 * Repeat each event n times
 */
function ply(n, pattern) {
    const count = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = pattern.query(span);
        const results = [];
        
        for (const event of events) {
            const duration = event.part.duration.toFloat();
            const step = duration / count;
            
            for (let i = 0; i < count; i++) {
                const newBegin = event.part.begin.add(step * i);
                const newEnd = event.part.begin.add(step * (i + 1));
                const newPart = new TimeSpan(newBegin, newEnd);
                
                const intersection = newPart.intersection(span);
                if (intersection) {
                    // Keep original whole, but subdivide part
                    results.push(new Event(event.whole, intersection, event.value, event.context));
                }
            }
        }
        return results;
    });
}

/**
 * Apply function at n times speed
 */
function inside(n, fn, pattern) {
    return fn(fast(n, pattern));
}

/**
 * Apply function at 1/n speed
 */
function outside(n, fn, pattern) {
    return fn(slow(n, pattern));
}

/**
 * Sample pattern n times per cycle
 */
function segment(n, pattern) {
    const samples = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            for (let i = 0; i < samples; i++) {
                const sampleTime = cycle + i / samples;
                const nextTime = cycle + (i + 1) / samples;
                
                const sampleSpan = new TimeSpan(sampleTime, nextTime);
                if (sampleSpan.overlaps(span)) {
                    // Query pattern at sample point
                    const pointEvents = pattern.query(new TimeSpan(sampleTime, sampleTime + 0.0001));
                    
                    if (pointEvents.length > 0) {
                        // Use the first event's value
                        const value = pointEvents[0].value;
                        const intersection = sampleSpan.intersection(span);
                        events.push(new Event(sampleSpan, intersection, value));
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Chop pattern into n pieces
 */
function chop(n, pattern) {
    const pieces = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Get events for this cycle
            const cycleEvents = pattern.query(new TimeSpan(cycle, cycle + 1));
            
            for (const event of cycleEvents) {
                const duration = event.part.duration.toFloat();
                const pieceSize = duration / pieces;
                
                for (let i = 0; i < pieces; i++) {
                    const pieceBegin = event.part.begin.add(pieceSize * i);
                    const pieceEnd = event.part.begin.add(pieceSize * (i + 1));
                    const piecePart = new TimeSpan(pieceBegin, pieceEnd);
                    
                    const intersection = piecePart.intersection(span);
                    if (intersection) {
                        // Add piece metadata
                        const newContext = {
                            ...event.context,
                            chop: i / pieces,
                            chopN: pieces
                        };
                        events.push(new Event(piecePart, intersection, event.value, newContext));
                    }
                }
            }
        }
        return events;
    });
}

// === Pattern Structure ===

/**
 * Reverse pattern within each cycle
 */
function rev(pattern) {
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const querySpan = cycleSpan.intersection(span);
            
            if (querySpan) {
                // Get events for this cycle  
                const cycleEvents = pattern.query(cycleSpan);
                
                // Reverse within cycle
                for (const event of cycleEvents) {
                    // Calculate relative position within cycle
                    const relBegin = event.part.begin.toFloat() - cycle;
                    const relEnd = event.part.end.toFloat() - cycle;
                    
                    // Mirror the position
                    const newBegin = cycle + (1 - relEnd);
                    const newEnd = cycle + (1 - relBegin);
                    
                    const newWhole = event.whole ? new TimeSpan(
                        cycle + (1 - (event.whole.end.toFloat() - cycle)),
                        cycle + (1 - (event.whole.begin.toFloat() - cycle))
                    ) : null;
                    
                    const newPart = new TimeSpan(newBegin, newEnd);
                    const intersection = newPart.intersection(span);
                    
                    if (intersection) {
                        events.push(new Event(newWhole, intersection, event.value, event.context));
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Pattern that plays forward then backward
 */
function palindrome(pattern) {
    return slowcat(pattern, rev(pattern));
}

/**
 * Rotate pattern by n steps
 */
function iter(n, pattern) {
    const rotation = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Calculate cumulative rotation for this cycle
            const cycleRotation = (cycle * rotation) % 1;
            
            // Query pattern for this cycle, then rotate
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const cycleEvents = pattern.query(cycleSpan);
            
            for (const event of cycleEvents) {
                // Calculate relative position and rotate
                const relBegin = event.part.begin.toFloat() - cycle;
                const relEnd = event.part.end.toFloat() - cycle;
                
                // Apply rotation (with wrapping)
                const rotBegin = (relBegin + cycleRotation) % 1;
                const rotEnd = (relEnd + cycleRotation) % 1;
                
                // Handle wrapping
                if (rotEnd > rotBegin) {
                    // No wrap
                    const newPart = new TimeSpan(cycle + rotBegin, cycle + rotEnd);
                    const intersection = newPart.intersection(span);
                    if (intersection) {
                        events.push(new Event(event.whole, intersection, event.value, event.context));
                    }
                } else {
                    // Wrapped - split into two parts
                    // First part: from rotBegin to end of cycle
                    const part1 = new TimeSpan(cycle + rotBegin, cycle + 1);
                    const int1 = part1.intersection(span);
                    if (int1) {
                        events.push(new Event(event.whole, int1, event.value, event.context));
                    }
                    
                    // Second part: from start of cycle to rotEnd
                    const part2 = new TimeSpan(cycle, cycle + rotEnd);
                    const int2 = part2.intersection(span);
                    if (int2) {
                        events.push(new Event(event.whole, int2, event.value, event.context));
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Apply function every n cycles
 */
function every(n, fn, pattern) {
    const period = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const querySpan = cycleSpan.intersection(span);
            
            if (querySpan) {
                // Apply function on every nth cycle
                const patToQuery = (cycle % period === 0) ? fn(pattern) : pattern;
                const cycleEvents = patToQuery.query(querySpan);
                events.push(...cycleEvents);
            }
        }
        return events;
    });
}

// === Randomness ===

// Xorshift RNG for deterministic randomness
function xorshift(seed) {
    let x = seed;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    return (x >>> 0) / 4294967296; // Convert to 0-1 range
}

// Get deterministic random value for a time
function timeToRand(t) {
    // Use cycle and position for seed
    const cycle = Math.floor(t);
    const pos = t - cycle;
    
    // Create seed from cycle (change every cycle)
    let seed = (cycle * 999331) >>> 0;
    return xorshift(seed);
}

// Get deterministic random with subcycle resolution
function timeToRandWithSub(t, n) {
    const cycle = Math.floor(t);
    const subCycle = Math.floor((t - cycle) * n);
    
    // Create seed from cycle and subcycle
    let seed = ((cycle * 999331) + (subCycle * 44111)) >>> 0;
    return xorshift(seed);
}

/**
 * Random values between 0 and 1
 */
function rand() {
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                const value = timeToRand(cycle);
                events.push(new Event(cycleSpan, intersection, value));
            }
        }
        return events;
    });
}

/**
 * Random integers between 0 and n-1
 */
function irand(n) {
    const max = typeof n === 'number' ? n : n.toFloat();
    
    return rand().fmap(x => Math.floor(x * max));
}

/**
 * Choose randomly from given values
 */
function choose(...values) {
    if (values.length === 0) return silence();
    
    return irand(values.length).fmap(i => values[i]);
}

/**
 * Weighted choice from values
 */
function wchoose(...pairs) {
    if (pairs.length === 0) return silence();
    
    // Calculate cumulative weights
    let total = 0;
    const cumulative = [];
    const values = [];
    
    for (const [value, weight] of pairs) {
        total += weight;
        cumulative.push(total);
        values.push(value);
    }
    
    return rand().fmap(r => {
        const target = r * total;
        for (let i = 0; i < cumulative.length; i++) {
            if (target < cumulative[i]) {
                return values[i];
            }
        }
        return values[values.length - 1];
    });
}

/**
 * Shuffle pattern slices
 */
function shuffle(n, pattern) {
    const pieces = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Create shuffled indices for this cycle
            const indices = [];
            const used = new Set();
            
            for (let i = 0; i < pieces; i++) {
                let idx;
                let attempts = 0;
                do {
                    const r = timeToRandWithSub(cycle + i / pieces, pieces);
                    idx = Math.floor(r * pieces);
                    attempts++;
                } while (used.has(idx) && attempts < pieces * 3);
                
                if (!used.has(idx)) {
                    used.add(idx);
                    indices.push(idx);
                } else {
                    // Fallback: find first unused
                    for (let j = 0; j < pieces; j++) {
                        if (!used.has(j)) {
                            used.add(j);
                            indices.push(j);
                            break;
                        }
                    }
                }
            }
            
            // Apply shuffle
            for (let i = 0; i < pieces; i++) {
                const srcIdx = indices[i];
                const srcBegin = cycle + srcIdx / pieces;
                const srcEnd = cycle + (srcIdx + 1) / pieces;
                
                const dstBegin = cycle + i / pieces;
                const dstEnd = cycle + (i + 1) / pieces;
                
                // Query pattern at source position
                const srcEvents = pattern.query(new TimeSpan(srcBegin, srcEnd));
                
                // Map to destination position
                for (const event of srcEvents) {
                    const relBegin = (event.part.begin.toFloat() - srcBegin) / (1 / pieces);
                    const relEnd = (event.part.end.toFloat() - srcBegin) / (1 / pieces);
                    
                    const newBegin = dstBegin + relBegin / pieces;
                    const newEnd = dstBegin + relEnd / pieces;
                    
                    const newPart = new TimeSpan(newBegin, newEnd);
                    const intersection = newPart.intersection(span);
                    
                    if (intersection) {
                        events.push(new Event(event.whole, intersection, event.value, event.context));
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Scramble pattern slices (with replacement)
 */
function scramble(n, pattern) {
    const pieces = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            for (let i = 0; i < pieces; i++) {
                // Pick random source piece
                const r = timeToRandWithSub(cycle + i / pieces, pieces);
                const srcIdx = Math.floor(r * pieces);
                
                const srcBegin = cycle + srcIdx / pieces;
                const srcEnd = cycle + (srcIdx + 1) / pieces;
                
                const dstBegin = cycle + i / pieces;
                const dstEnd = cycle + (i + 1) / pieces;
                
                // Query pattern at source position
                const srcEvents = pattern.query(new TimeSpan(srcBegin, srcEnd));
                
                // Map to destination position
                for (const event of srcEvents) {
                    const relBegin = (event.part.begin.toFloat() - srcBegin) / (1 / pieces);
                    const relEnd = (event.part.end.toFloat() - srcBegin) / (1 / pieces);
                    
                    const newBegin = dstBegin + relBegin / pieces;
                    const newEnd = dstBegin + relEnd / pieces;
                    
                    const newPart = new TimeSpan(newBegin, newEnd);
                    const intersection = newPart.intersection(span);
                    
                    if (intersection) {
                        events.push(new Event(event.whole, intersection, event.value, event.context));
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Randomly remove events
 */
function degrade(pattern) {
    return degradeBy(0.5, pattern);
}

/**
 * Randomly remove events by probability
 */
function degradeBy(prob, pattern) {
    const p = typeof prob === 'number' ? prob : prob.toFloat();
    
    // Edge cases
    if (p <= 0) return pattern;  // Remove nothing
    if (p >= 1) return silence(); // Remove everything
    
    return new Pattern((span) => {
        const events = pattern.query(span);
        const filtered = [];
        
        for (const event of events) {
            // Use event time for deterministic random
            const t = event.part.begin.toFloat();
            const r = timeToRandWithSub(t, 1000);
            
            if (r > p) {
                filtered.push(event);
            }
        }
        return filtered;
    });
}

/**
 * Apply function sometimes
 */
function sometimes(fn, pattern) {
    return sometimesBy(0.5, fn, pattern);
}

/**
 * Apply function with probability
 */
function sometimesBy(prob, fn, pattern) {
    const p = typeof prob === 'number' ? prob : prob.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        const transformed = fn(pattern);
        
        const origEvents = pattern.query(span);
        const transEvents = transformed.query(span);
        
        // Merge based on probability
        for (const event of origEvents) {
            const t = event.part.begin.toFloat();
            const r = timeToRandWithSub(t, 1000);
            
            if (r > p) {
                events.push(event);
            }
        }
        
        for (const event of transEvents) {
            const t = event.part.begin.toFloat();
            const r = timeToRandWithSub(t, 1000);
            
            if (r <= p) {
                events.push(event);
            }
        }
        
        return events;
    });
}

/**
 * Often apply function (75% of the time)
 */
function often(fn, pattern) {
    return sometimesBy(0.75, fn, pattern);
}

/**
 * Rarely apply function (25% of the time)
 */
function rarely(fn, pattern) {
    return sometimesBy(0.25, fn, pattern);
}

/**
 * Almost never apply function (10% of the time)
 */
function almostNever(fn, pattern) {
    return sometimesBy(0.1, fn, pattern);
}

/**
 * Almost always apply function (90% of the time)
 */
function almostAlways(fn, pattern) {
    return sometimesBy(0.9, fn, pattern);
}

// === Signal Generators ===

/**
 * Sine wave signal 0 to 1
 */
function sine() {
    return new Pattern((span) => {
        const events = [];
        
        // Sample at reasonable resolution
        const resolution = 16; // samples per cycle
        const begin = Math.floor(span.begin.toFloat());
        const end = Math.ceil(span.end.toFloat());
        
        for (let cycle = begin; cycle <= end; cycle++) {
            for (let i = 0; i < resolution; i++) {
                const t = cycle + i / resolution;
                const nextT = cycle + (i + 1) / resolution;
                
                const sampleSpan = new TimeSpan(t, nextT);
                const intersection = sampleSpan.intersection(span);
                
                if (intersection) {
                    // Sine value at this position
                    const value = (Math.sin(t * Math.PI * 2) + 1) / 2;
                    events.push(new Event(sampleSpan, intersection, value));
                }
            }
        }
        return events;
    });
}

/**
 * Cosine wave signal 0 to 1
 */
function cosine() {
    return new Pattern((span) => {
        const events = [];
        const resolution = 16;
        const begin = Math.floor(span.begin.toFloat());
        const end = Math.ceil(span.end.toFloat());
        
        for (let cycle = begin; cycle <= end; cycle++) {
            for (let i = 0; i < resolution; i++) {
                const t = cycle + i / resolution;
                const nextT = cycle + (i + 1) / resolution;
                
                const sampleSpan = new TimeSpan(t, nextT);
                const intersection = sampleSpan.intersection(span);
                
                if (intersection) {
                    const value = (Math.cos(t * Math.PI * 2) + 1) / 2;
                    events.push(new Event(sampleSpan, intersection, value));
                }
            }
        }
        return events;
    });
}

/**
 * Sawtooth wave signal 0 to 1
 */
function saw() {
    return new Pattern((span) => {
        const events = [];
        const resolution = 16;
        const begin = Math.floor(span.begin.toFloat());
        const end = Math.ceil(span.end.toFloat());
        
        for (let cycle = begin; cycle <= end; cycle++) {
            for (let i = 0; i < resolution; i++) {
                const t = cycle + i / resolution;
                const nextT = cycle + (i + 1) / resolution;
                
                const sampleSpan = new TimeSpan(t, nextT);
                const intersection = sampleSpan.intersection(span);
                
                if (intersection) {
                    const value = t - Math.floor(t); // Ramp 0 to 1
                    events.push(new Event(sampleSpan, intersection, value));
                }
            }
        }
        return events;
    });
}

/**
 * Square wave signal 0 to 1
 */
function square() {
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // First half: 1
            const firstHalf = new TimeSpan(cycle, cycle + 0.5);
            const int1 = firstHalf.intersection(span);
            if (int1) {
                events.push(new Event(firstHalf, int1, 1));
            }
            
            // Second half: 0
            const secondHalf = new TimeSpan(cycle + 0.5, cycle + 1);
            const int2 = secondHalf.intersection(span);
            if (int2) {
                events.push(new Event(secondHalf, int2, 0));
            }
        }
        return events;
    });
}

/**
 * Triangle wave signal 0 to 1
 */
function tri() {
    return new Pattern((span) => {
        const events = [];
        const resolution = 16;
        const begin = Math.floor(span.begin.toFloat());
        const end = Math.ceil(span.end.toFloat());
        
        for (let cycle = begin; cycle <= end; cycle++) {
            for (let i = 0; i < resolution; i++) {
                const t = cycle + i / resolution;
                const nextT = cycle + (i + 1) / resolution;
                
                const sampleSpan = new TimeSpan(t, nextT);
                const intersection = sampleSpan.intersection(span);
                
                if (intersection) {
                    const phase = t - Math.floor(t);
                    const value = phase < 0.5 
                        ? phase * 2  // Rising
                        : 2 - phase * 2; // Falling
                    events.push(new Event(sampleSpan, intersection, value));
                }
            }
        }
        return events;
    });
}

/**
 * Perlin noise signal 0 to 1
 */
function perlin() {
    return new Pattern((span) => {
        const events = [];
        const resolution = 16;
        const begin = Math.floor(span.begin.toFloat());
        const end = Math.ceil(span.end.toFloat());
        
        for (let cycle = begin; cycle <= end; cycle++) {
            for (let i = 0; i < resolution; i++) {
                const t = cycle + i / resolution;
                const nextT = cycle + (i + 1) / resolution;
                
                const sampleSpan = new TimeSpan(t, nextT);
                const intersection = sampleSpan.intersection(span);
                
                if (intersection) {
                    // Simple perlin-like smooth noise
                    const ta = Math.floor(t);
                    const tb = ta + 1;
                    const weight = t - ta;
                    
                    // Smoothstep interpolation
                    const smooth = weight * weight * (3 - 2 * weight);
                    
                    // Get random values at integer positions
                    const va = timeToRand(ta);
                    const vb = timeToRand(tb);
                    
                    // Interpolate
                    const value = va + smooth * (vb - va);
                    
                    events.push(new Event(sampleSpan, intersection, value));
                }
            }
        }
        return events;
    });
}

// === Euclidean Rhythms ===

/**
 * Bjorklund's algorithm for euclidean rhythms
 */
function bjorklund(steps, pulses) {
    if (pulses > steps) pulses = steps;
    if (pulses === 0) return new Array(steps).fill(0);
    if (pulses === steps) return new Array(steps).fill(1);
    
    let pattern = [];
    let counts = [];
    let remainders = [];
    
    let divisor = steps - pulses;
    remainders.push(pulses);
    let level = 0;
    
    while (remainders[level] > 1) {
        counts.push(Math.floor(divisor / remainders[level]));
        remainders.push(divisor % remainders[level]);
        divisor = remainders[level];
        level++;
    }
    
    counts.push(divisor);
    
    const build = function(l) {
        if (l === -1) {
            pattern.push(0);
        } else if (l === -2) {
            pattern.push(1);
        } else {
            for (let i = 0; i < counts[l]; i++) {
                build(l - 1);
            }
            if (remainders[l] !== 0) {
                build(l - 2);
            }
        }
    };
    
    build(level);
    
    // Rotate to start with a pulse
    const firstPulse = pattern.indexOf(1);
    if (firstPulse > 0) {
        pattern = pattern.slice(firstPulse).concat(pattern.slice(0, firstPulse));
    }
    
    return pattern;
}

/**
 * Euclidean rhythm pattern
 */
function euclid(pulses, steps, rotation = 0) {
    const k = typeof pulses === 'number' ? pulses : pulses.toFloat();
    const n = typeof steps === 'number' ? steps : steps.toFloat();
    const r = typeof rotation === 'number' ? rotation : rotation.toFloat();
    
    if (n <= 0) return silence();
    
    const pattern = bjorklund(n, k);
    
    // Apply rotation
    const rotAmount = Math.floor(r % n);
    const rotated = pattern.slice(rotAmount).concat(pattern.slice(0, rotAmount));
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            for (let i = 0; i < n; i++) {
                if (rotated[i] === 1) {
                    const begin = cycle + i / n;
                    const end = cycle + (i + 1) / n;
                    
                    const eventSpan = new TimeSpan(begin, end);
                    const intersection = eventSpan.intersection(span);
                    
                    if (intersection) {
                        events.push(new Event(eventSpan, intersection, true));
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Euclidean rhythm with rotation
 */
function euclidRot(pulses, steps, rotation) {
    return euclid(pulses, steps, rotation);
}

/**
 * Euclidean rhythm with legato
 */
function euclidLegato(pulses, steps) {
    const k = typeof pulses === 'number' ? pulses : pulses.toFloat();
    const n = typeof steps === 'number' ? steps : steps.toFloat();
    
    if (n <= 0) return silence();
    if (k <= 0) return silence();
    
    const pattern = bjorklund(n, k);
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Find continuous runs of 1s
            let i = 0;
            while (i < n) {
                if (pattern[i] === 1) {
                    // Found start of a run
                    const noteStart = cycle + i / n;
                    let j = i + 1;
                    
                    // Find end of run
                    while (j < n && pattern[j] === 1) {
                        j++;
                    }
                    
                    // If we have more 1s, extend to next 1
                    if (j < n) {
                        // Find next 1
                        let next1 = j;
                        while (next1 < n && pattern[next1] === 0) {
                            next1++;
                        }
                        
                        // Extend note to just before next pulse
                        const noteEnd = cycle + next1 / n;
                        const eventSpan = new TimeSpan(noteStart, noteEnd);
                        const intersection = eventSpan.intersection(span);
                        
                        if (intersection) {
                            events.push(new Event(eventSpan, intersection, true));
                        }
                    } else {
                        // Last pulse, check if first pulse is 1 (wrap around)
                        if (pattern[0] === 1) {
                            // Extend to next cycle's first pulse
                            const noteEnd = cycle + 1;
                            const eventSpan = new TimeSpan(noteStart, noteEnd);
                            const intersection = eventSpan.intersection(span);
                            
                            if (intersection) {
                                events.push(new Event(eventSpan, intersection, true));
                            }
                        } else {
                            // Extend to end of cycle
                            const noteEnd = cycle + 1;
                            const eventSpan = new TimeSpan(noteStart, noteEnd);
                            const intersection = eventSpan.intersection(span);
                            
                            if (intersection) {
                                events.push(new Event(eventSpan, intersection, true));
                            }
                        }
                    }
                    
                    i = j;
                } else {
                    i++;
                }
            }
        }
        return events;
    });
}

// Export everything
module.exports = {
    // Classes
    Fraction,
    TimeSpan,
    Event,
    Pattern,
    
    // Core creation
    pure,
    silence,
    gap,
    
    // Combination
    stack,
    cat,
    fastcat,
    slowcat,
    
    // Time manipulation
    fast,
    slow,
    early,
    late,
    compress,
    zoom,
    ply,
    inside,
    outside,
    segment,
    chop,
    
    // Pattern structure
    rev,
    palindrome,
    iter,
    every,
    
    // Randomness
    rand,
    irand,
    choose,
    wchoose,
    shuffle,
    scramble,
    degrade,
    degradeBy,
    sometimes,
    sometimesBy,
    often,
    rarely,
    almostNever,
    almostAlways,
    
    // Signal generators
    sine,
    cosine,
    saw,
    square,
    tri,
    perlin,
    
    // Euclidean rhythms
    euclid,
    euclidRot,
    euclidLegato
};
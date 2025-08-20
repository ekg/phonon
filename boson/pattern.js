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

// === Pattern Combination ===

/**
 * Stereo split - apply function to one channel
 */
function jux(fn, pattern) {
    return new Pattern((span) => {
        const leftEvents = pattern.query(span);
        const rightEvents = fn(pattern).query(span);
        
        // Add pan metadata
        const events = [];
        for (const event of leftEvents) {
            events.push(new Event(event.whole, event.part, event.value, 
                { ...event.context, pan: 0 }));
        }
        for (const event of rightEvents) {
            events.push(new Event(event.whole, event.part, event.value,
                { ...event.context, pan: 1 }));
        }
        return events;
    });
}

/**
 * Jux with configurable pan amount
 */
function juxBy(amount, fn, pattern) {
    const amt = typeof amount === 'number' ? amount : amount.toFloat();
    
    return new Pattern((span) => {
        const leftEvents = pattern.query(span);
        const rightEvents = fn(pattern).query(span);
        
        const events = [];
        for (const event of leftEvents) {
            events.push(new Event(event.whole, event.part, event.value,
                { ...event.context, pan: 0.5 - amt/2 }));
        }
        for (const event of rightEvents) {
            events.push(new Event(event.whole, event.part, event.value,
                { ...event.context, pan: 0.5 + amt/2 }));
        }
        return events;
    });
}

/**
 * Layer pattern with transformed version
 */
function superimpose(fn, pattern) {
    return stack(pattern, fn(pattern));
}

/**
 * Layer multiple transformations
 */
function layer(...fns) {
    return function(pattern) {
        const patterns = fns.map(fn => fn(pattern));
        return stack(pattern, ...patterns);
    };
}

/**
 * Offset and layer
 */
function off(time, fn, pattern) {
    const t = typeof time === 'number' ? time : time.toFloat();
    return stack(pattern, fn(late(t, pattern)));
}

/**
 * Echo effect
 */
function echo(n, time, feedback, pattern) {
    const count = typeof n === 'number' ? n : n.toFloat();
    const delay = typeof time === 'number' ? time : time.toFloat();
    const fb = typeof feedback === 'number' ? feedback : feedback.toFloat();
    
    const patterns = [pattern];
    let gain = fb;
    
    for (let i = 1; i <= count; i++) {
        const delayed = late(delay * i, pattern);
        // Apply gain reduction
        const withGain = delayed.fmap(v => {
            if (typeof v === 'object' && v !== null) {
                return { ...v, gain: (v.gain || 1) * gain };
            }
            return v;
        });
        patterns.push(withGain);
        gain *= fb;
    }
    
    return stack(...patterns);
}

/**
 * Stutter effect
 */
function stut(n, feedback, time, pattern) {
    return echo(n, time, feedback, pattern);
}

// === Filtering & Masking ===

/**
 * Apply function when test is true
 */
function when(test, fn, pattern) {
    return new Pattern((span) => {
        const events = pattern.query(span);
        const results = [];
        
        for (const event of events) {
            // Evaluate test for this event
            const testResult = typeof test === 'function' 
                ? test(event.value)
                : test;
            
            if (testResult) {
                // Apply function
                const transformed = fn(pure(event.value)).query(event.part);
                for (const tEvent of transformed) {
                    results.push(new Event(event.whole, tEvent.part, tEvent.value, 
                        { ...event.context, ...tEvent.context }));
                }
            } else {
                results.push(event);
            }
        }
        return results;
    });
}

/**
 * Boolean mask pattern
 */
function mask(maskPattern, pattern) {
    return new Pattern((span) => {
        const patternEvents = pattern.query(span);
        const maskEvents = maskPattern.query(span);
        const results = [];
        
        for (const pEvent of patternEvents) {
            // Check if any mask event overlaps
            let masked = false;
            for (const mEvent of maskEvents) {
                if (pEvent.part.overlaps(mEvent.part) && mEvent.value) {
                    masked = true;
                    break;
                }
            }
            
            if (masked) {
                results.push(pEvent);
            }
        }
        return results;
    });
}

/**
 * Apply structure from one pattern to another
 */
function struct(structPattern, pattern) {
    return new Pattern((span) => {
        const structEvents = structPattern.query(span);
        const results = [];
        
        for (const sEvent of structEvents) {
            // Query pattern at this position
            const patternEvents = pattern.query(sEvent.part);
            
            if (patternEvents.length > 0) {
                // Use first event's value
                const value = patternEvents[0].value;
                results.push(new Event(sEvent.whole, sEvent.part, value, sEvent.context));
            }
        }
        return results;
    });
}

/**
 * Filter events by predicate
 */
function filter(predicate, pattern) {
    return new Pattern((span) => {
        const events = pattern.query(span);
        return events.filter(event => predicate(event.value));
    });
}

// === Math Operations ===

/**
 * Add to pattern values
 */
function add(n, pattern) {
    const val = typeof n === 'number' ? n : n.toFloat();
    return pattern.fmap(v => {
        if (typeof v === 'number') return v + val;
        if (v && typeof v.toFloat === 'function') return v.toFloat() + val;
        return v;
    });
}

/**
 * Subtract from pattern values
 */
function sub(n, pattern) {
    const val = typeof n === 'number' ? n : n.toFloat();
    return pattern.fmap(v => {
        if (typeof v === 'number') return v - val;
        if (v && typeof v.toFloat === 'function') return v.toFloat() - val;
        return v;
    });
}

/**
 * Multiply pattern values
 */
function mul(n, pattern) {
    const val = typeof n === 'number' ? n : n.toFloat();
    return pattern.fmap(v => {
        if (typeof v === 'number') return v * val;
        if (v && typeof v.toFloat === 'function') return v.toFloat() * val;
        return v;
    });
}

/**
 * Divide pattern values
 */
function div(n, pattern) {
    const val = typeof n === 'number' ? n : n.toFloat();
    return pattern.fmap(v => {
        if (typeof v === 'number') return v / val;
        if (v && typeof v.toFloat === 'function') return v.toFloat() / val;
        return v;
    });
}

/**
 * Modulo pattern values
 */
function mod(n, pattern) {
    const val = typeof n === 'number' ? n : n.toFloat();
    return pattern.fmap(v => {
        if (typeof v === 'number') return v % val;
        if (v && typeof v.toFloat === 'function') return v.toFloat() % val;
        return v;
    });
}

/**
 * Map values to range
 */
function range(min, max, pattern) {
    const minVal = typeof min === 'number' ? min : min.toFloat();
    const maxVal = typeof max === 'number' ? max : max.toFloat();
    const span = maxVal - minVal;
    
    return pattern.fmap(v => {
        let normalized;
        if (typeof v === 'number') {
            normalized = v;
        } else if (v && typeof v.toFloat === 'function') {
            normalized = v.toFloat();
        } else {
            return v;
        }
        
        // Assume input is 0-1
        return minVal + normalized * span;
    });
}

// === Additional Core Patterns ===

/**
 * Sequential pattern
 */
function sequence(...patterns) {
    if (patterns.length === 0) return silence();
    
    return new Pattern((span) => {
        const events = [];
        const n = patterns.length;
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Play each pattern for 1/n of cycle
            for (let i = 0; i < n; i++) {
                const sliceBegin = cycle + i / n;
                const sliceEnd = cycle + (i + 1) / n;
                const sliceSpan = new TimeSpan(sliceBegin, sliceEnd);
                
                if (sliceSpan.overlaps(span)) {
                    const patternEvents = patterns[i].query(new TimeSpan(0, 1));
                    
                    for (const event of patternEvents) {
                        const scaledPart = new TimeSpan(
                            sliceBegin + event.part.begin.toFloat() / n,
                            sliceBegin + event.part.end.toFloat() / n
                        );
                        
                        const intersection = scaledPart.intersection(span);
                        if (intersection) {
                            events.push(new Event(event.whole, intersection, event.value, event.context));
                        }
                    }
                }
            }
        }
        return events;
    });
}

/**
 * Polymeter - patterns with different lengths
 */
function polymeter(...patterns) {
    if (patterns.length === 0) return silence();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let i = 0; i < patterns.length; i++) {
            const pattern = patterns[i];
            const patternEvents = pattern.query(span);
            events.push(...patternEvents);
        }
        
        return events;
    });
}

/**
 * Polyrhythm - patterns at different speeds
 */
function polyrhythm(...patterns) {
    if (patterns.length === 0) return silence();
    
    const stacked = [];
    for (let i = 0; i < patterns.length; i++) {
        // Speed up each pattern to fit
        stacked.push(fast(patterns.length, patterns[i]));
    }
    
    return stack(...stacked);
}

// === Additional Pattern Structure ===

/**
 * Apply function on first of n cycles
 */
function firstOf(n, fn, pattern) {
    const period = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const querySpan = cycleSpan.intersection(span);
            
            if (querySpan) {
                // Apply function only on first cycle of each period
                const patToQuery = (cycle % period === 0) ? fn(pattern) : pattern;
                const cycleEvents = patToQuery.query(querySpan);
                events.push(...cycleEvents);
            }
        }
        return events;
    });
}

/**
 * Apply function on last of n cycles
 */
function lastOf(n, fn, pattern) {
    const period = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const querySpan = cycleSpan.intersection(span);
            
            if (querySpan) {
                // Apply function only on last cycle of each period
                const patToQuery = (cycle % period === period - 1) ? fn(pattern) : pattern;
                const cycleEvents = patToQuery.query(querySpan);
                events.push(...cycleEvents);
            }
        }
        return events;
    });
}

/**
 * Brak - creates a syncopated half-time feel
 */
function brak(pattern) {
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const isEven = cycle % 2 === 0;
            
            if (isEven) {
                // Normal on even cycles
                const cycleEvents = pattern.query(new TimeSpan(cycle, cycle + 1));
                for (const event of cycleEvents) {
                    const intersection = event.part.intersection(span);
                    if (intersection) {
                        events.push(new Event(event.whole, intersection, event.value, event.context));
                    }
                }
            } else {
                // Syncopated on odd cycles - shift by 1/4
                const shifted = late(0.25, pattern);
                const cycleEvents = shifted.query(new TimeSpan(cycle, cycle + 1));
                for (const event of cycleEvents) {
                    const intersection = event.part.intersection(span);
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
 * Press - compress events to start of cycle
 */
function press(pattern) {
    return compress(0, 0.5, pattern);
}

/**
 * Hurry - speed up pattern and pitch
 */
function hurry(factor, pattern) {
    const f = typeof factor === 'number' ? factor : factor.toFloat();
    
    return fast(f, pattern).fmap(v => {
        // Add pitch metadata for speed change
        if (typeof v === 'object' && v !== null) {
            return { ...v, speed: (v.speed || 1) * f };
        }
        return v;
    });
}

/**
 * Take nth bite/slice of pattern
 */
function bite(n, i, pattern) {
    const slices = typeof n === 'number' ? n : n.toFloat();
    const index = typeof i === 'number' ? i : i.toFloat();
    
    if (index < 0 || index >= slices) return silence();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Get only the specified slice
            const sliceBegin = cycle + index / slices;
            const sliceEnd = cycle + (index + 1) / slices;
            const sliceSpan = new TimeSpan(sliceBegin, sliceEnd);
            
            if (sliceSpan.overlaps(span)) {
                const patEvents = pattern.query(new TimeSpan(index / slices, (index + 1) / slices));
                
                for (const event of patEvents) {
                    // Map to the slice position in this cycle
                    const scaledPart = new TimeSpan(
                        cycle + event.part.begin.toFloat(),
                        cycle + event.part.end.toFloat()
                    );
                    
                    const intersection = scaledPart.intersection(span);
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
 * Striate - interleave slices of pattern
 */
function striate(n, pattern) {
    const slices = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Query full pattern
            const patEvents = pattern.query(new TimeSpan(cycle, cycle + 1));
            
            // Sort events for consistent ordering
            patEvents.sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
            
            // Create slices for each event  
            const allSlices = [];
            for (let j = 0; j < patEvents.length; j++) {
                const event = patEvents[j];
                const eventDuration = event.part.duration.toFloat();
                const sliceSize = eventDuration / slices;
                
                for (let i = 0; i < slices; i++) {
                    allSlices.push({
                        event: event,
                        eventIndex: j,
                        sliceIndex: i,
                        sliceSize: sliceSize
                    });
                }
            }
            
            // Interleave the slices
            const totalSlices = allSlices.length;
            const sliceSpan = 1 / totalSlices;
            
            for (let i = 0; i < totalSlices; i++) {
                const slice = allSlices[i];
                const dstBegin = cycle + i * sliceSpan;
                const dstEnd = dstBegin + sliceSpan;
                
                const dstSpan = new TimeSpan(dstBegin, dstEnd);
                const intersection = dstSpan.intersection(span);
                
                if (intersection) {
                    const newContext = {
                        ...slice.event.context,
                        striate: slice.sliceIndex / slices,
                        striateN: slices
                    };
                    events.push(new Event(slice.event.whole, intersection, slice.event.value, newContext));
                }
            }
        }
        return events;
    });
}

/**
 * Inhabit - fill structure with pattern values
 */
function inhabit(structPattern, valuePattern) {
    return struct(structPattern, valuePattern);
}

/**
 * Arpeggiate pattern
 */
function arp(mode, pattern) {
    const modes = {
        'up': (vals) => vals,
        'down': (vals) => vals.slice().reverse(),
        'updown': (vals) => vals.concat(vals.slice(1, -1).reverse()),
        'downup': (vals) => vals.slice().reverse().concat(vals.slice(1, -1)),
        'converge': (vals) => {
            const result = [];
            let left = 0, right = vals.length - 1;
            while (left <= right) {
                if (left === right) {
                    result.push(vals[left]);
                } else {
                    result.push(vals[left], vals[right]);
                }
                left++;
                right--;
            }
            return result;
        },
        'diverge': (vals) => {
            const mid = Math.floor(vals.length / 2);
            const result = [];
            for (let i = 0; i <= mid; i++) {
                if (mid - i >= 0) result.push(vals[mid - i]);
                if (mid + i < vals.length && i > 0) result.push(vals[mid + i]);
            }
            return result;
        }
    };
    
    const arpFn = modes[mode] || modes['up'];
    
    return new Pattern((span) => {
        const events = pattern.query(span);
        const results = [];
        
        for (const event of events) {
            // If event value is an array, arpeggiate it
            if (Array.isArray(event.value)) {
                const arped = arpFn(event.value);
                const stepDuration = event.part.duration.toFloat() / arped.length;
                
                for (let i = 0; i < arped.length; i++) {
                    const stepBegin = event.part.begin.add(stepDuration * i);
                    const stepEnd = event.part.begin.add(stepDuration * (i + 1));
                    const stepSpan = new TimeSpan(stepBegin, stepEnd);
                    
                    const intersection = stepSpan.intersection(span);
                    if (intersection) {
                        results.push(new Event(event.whole, intersection, arped[i], event.context));
                    }
                }
            } else {
                results.push(event);
            }
        }
        return results;
    });
}

/**
 * Custom arpeggiation with function
 */
function arpWith(fn, pattern) {
    return new Pattern((span) => {
        const events = pattern.query(span);
        const results = [];
        
        for (const event of events) {
            if (Array.isArray(event.value)) {
                const arped = fn(event.value);
                const stepDuration = event.part.duration.toFloat() / arped.length;
                
                for (let i = 0; i < arped.length; i++) {
                    const stepBegin = event.part.begin.add(stepDuration * i);
                    const stepEnd = event.part.begin.add(stepDuration * (i + 1));
                    const stepSpan = new TimeSpan(stepBegin, stepEnd);
                    
                    const intersection = stepSpan.intersection(span);
                    if (intersection) {
                        results.push(new Event(event.whole, intersection, arped[i], event.context));
                    }
                }
            } else {
                results.push(event);
            }
        }
        return results;
    });
}

/**
 * Exponential range mapping
 */
function rangex(min, max, pattern) {
    const minVal = typeof min === 'number' ? min : min.toFloat();
    const maxVal = typeof max === 'number' ? max : max.toFloat();
    
    return pattern.fmap(v => {
        let normalized;
        if (typeof v === 'number') {
            normalized = v;
        } else if (v && typeof v.toFloat === 'function') {
            normalized = v.toFloat();
        } else {
            return v;
        }
        
        // Exponential mapping (assuming input 0-1)
        const exp = Math.pow(2, normalized * 10) / 1024; // 2^(x*10)/1024 maps 0->~0, 1->1
        return minVal + exp * (maxVal - minVal);
    });
}

/**
 * Fit pattern to n steps
 */
function fit(n, pattern) {
    const steps = typeof n === 'number' ? n : n.toFloat();
    return fast(steps, pattern);
}

/**
 * Take first n events
 */
function take(n, pattern) {
    const count = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = pattern.query(span);
        
        // Count events per cycle
        const cycleEvents = {};
        for (const event of events) {
            const cycle = Math.floor(event.part.begin.toFloat());
            if (!cycleEvents[cycle]) cycleEvents[cycle] = [];
            cycleEvents[cycle].push(event);
        }
        
        // Take first n from each cycle
        const results = [];
        for (const cycle in cycleEvents) {
            const sorted = cycleEvents[cycle].sort((a, b) => 
                a.part.begin.toFloat() - b.part.begin.toFloat()
            );
            results.push(...sorted.slice(0, count));
        }
        
        return results;
    });
}

/**
 * Drop first n events
 */
function drop(n, pattern) {
    const count = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = pattern.query(span);
        
        // Count events per cycle
        const cycleEvents = {};
        for (const event of events) {
            const cycle = Math.floor(event.part.begin.toFloat());
            if (!cycleEvents[cycle]) cycleEvents[cycle] = [];
            cycleEvents[cycle].push(event);
        }
        
        // Drop first n from each cycle
        const results = [];
        for (const cycle in cycleEvents) {
            const sorted = cycleEvents[cycle].sort((a, b) => 
                a.part.begin.toFloat() - b.part.begin.toFloat()
            );
            results.push(...sorted.slice(count));
        }
        
        return results;
    });
}

/**
 * Run - sequence of numbers
 */
function run(n) {
    const count = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            for (let i = 0; i < count; i++) {
                const begin = cycle + i / count;
                const end = cycle + (i + 1) / count;
                const eventSpan = new TimeSpan(begin, end);
                
                const intersection = eventSpan.intersection(span);
                if (intersection) {
                    events.push(new Event(eventSpan, intersection, i));
                }
            }
        }
        return events;
    });
}

/**
 * Steps - set target step count for pattern
 */
function steps(n, pattern) {
    const stepCount = typeof n === 'number' ? n : n.toFloat();
    return segment(stepCount, pattern);
}

/**
 * SomeCycles - Apply function to some cycles (randomly per cycle)
 */
function someCycles(fn, pattern) {
    return someCyclesBy(0.5, fn, pattern);
}

/**
 * SomeCyclesBy - Apply function to some cycles with probability
 */
function someCyclesBy(prob, fn, pattern) {
    const p = typeof prob === 'number' ? prob : prob.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            // Use cycle-based seed for deterministic randomness
            const seed = cycle * 999331;
            const rng = xorshift(seed);
            const rand = (rng() & 0xFFFF) / 0xFFFF;
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                if (rand < p) {
                    // Apply function this cycle
                    events.push(...fn(pattern).query(intersection));
                } else {
                    // Use original pattern this cycle
                    events.push(...pattern.query(intersection));
                }
            }
        }
        
        return events;
    });
}

/**
 * Swing - Add swing timing
 */
function swing(pattern) {
    return swingBy(0.05, pattern);
}

/**
 * SwingBy - Add swing timing with amount
 */
function swingBy(amount, pattern) {
    const amt = typeof amount === 'number' ? amount : amount.toFloat();
    
    return new Pattern((span) => {
        const events = pattern.query(span);
        
        return events.map(event => {
            const pos = event.part.begin.toFloat();
            const cycle = Math.floor(pos);
            const phase = pos - cycle;
            
            // Apply swing to off-beats
            let adjustment = 0;
            const beatPos = (phase * 4) % 1;
            if (beatPos > 0.4 && beatPos < 0.6) {
                adjustment = amt;
            }
            
            const newBegin = event.part.begin.add(adjustment);
            const newEnd = event.part.end.add(adjustment);
            
            return new Event(
                event.whole,
                new TimeSpan(newBegin, newEnd),
                event.value,
                event.context
            );
        });
    });
}

/**
 * Grid - Create grid pattern
 */
function grid(n, pattern) {
    const gridSize = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const stepSize = 1 / gridSize;
            
            for (let i = 0; i < gridSize; i++) {
                const begin = cycle + i * stepSize;
                const end = begin + stepSize;
                const gridSpan = new TimeSpan(begin, end);
                const intersection = gridSpan.intersection(span);
                
                if (intersection) {
                    const patEvents = pattern.query(new TimeSpan(0, 1));
                    for (const e of patEvents) {
                        events.push(new Event(
                            gridSpan,
                            intersection,
                            e.value,
                            e.context
                        ));
                    }
                }
            }
        }
        
        return events;
    });
}

/**
 * Place - Place values at specific positions
 */
function place(positions, values) {
    return new Pattern((span) => {
        const events = [];
        const posArray = Array.isArray(positions) ? positions : [positions];
        const valArray = Array.isArray(values) ? values : [values];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            for (let i = 0; i < posArray.length && i < valArray.length; i++) {
                const pos = typeof posArray[i] === 'number' ? posArray[i] : posArray[i].toFloat();
                const begin = cycle + pos;
                const end = begin + 0.1; // Small duration
                
                const eventSpan = new TimeSpan(begin, end);
                const intersection = eventSpan.intersection(span);
                
                if (intersection) {
                    events.push(new Event(eventSpan, intersection, valArray[i]));
                }
            }
        }
        
        return events;
    });
}

/**
 * Binary - Create pattern from binary number
 */
function binary(n, value = 1) {
    const num = typeof n === 'number' ? n : n.toFloat();
    const bits = num.toString(2);
    
    return new Pattern((span) => {
        const events = [];
        const stepSize = 1 / bits.length;
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            for (let i = 0; i < bits.length; i++) {
                if (bits[i] === '1') {
                    const begin = cycle + i * stepSize;
                    const end = begin + stepSize;
                    const eventSpan = new TimeSpan(begin, end);
                    const intersection = eventSpan.intersection(span);
                    
                    if (intersection) {
                        events.push(new Event(eventSpan, intersection, value));
                    }
                }
            }
        }
        
        return events;
    });
}

/**
 * ASCII - Create pattern from ASCII string
 */
function ascii(str) {
    return new Pattern((span) => {
        const events = [];
        const stepSize = 1 / str.length;
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            for (let i = 0; i < str.length; i++) {
                const begin = cycle + i * stepSize;
                const end = begin + stepSize;
                const eventSpan = new TimeSpan(begin, end);
                const intersection = eventSpan.intersection(span);
                
                if (intersection) {
                    events.push(new Event(
                        eventSpan,
                        intersection,
                        str.charCodeAt(i)
                    ));
                }
            }
        }
        
        return events;
    });
}

/**
 * Saw2 - Bipolar sawtooth wave (-1 to 1)
 */
function saw2() {
    return saw().mul(2).sub(1);
}

/**
 * Sine2 - Bipolar sine wave (-1 to 1)
 */
function sine2() {
    return sine().mul(2).sub(1);
}

/**
 * Square2 - Bipolar square wave (-1 to 1)
 */
function square2() {
    return square().mul(2).sub(1);
}

/**
 * Tri2 - Bipolar triangle wave (-1 to 1)
 */
function tri2() {
    return tri().mul(2).sub(1);
}

/**
 * Weave - Weave patterns together
 */
function weave(n, patterns) {
    const density = typeof n === 'number' ? n : n.toFloat();
    const pats = Array.isArray(patterns) ? patterns : [patterns];
    
    return new Pattern((span) => {
        const events = [];
        const sliceSize = 1 / (density * pats.length);
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            let pos = 0;
            for (let i = 0; i < density; i++) {
                for (let j = 0; j < pats.length; j++) {
                    const begin = cycle + pos;
                    const end = begin + sliceSize;
                    pos += sliceSize;
                    
                    const sliceSpan = new TimeSpan(begin, end);
                    const intersection = sliceSpan.intersection(span);
                    
                    if (intersection) {
                        const patEvents = pats[j].query(new TimeSpan(i / density, (i + 1) / density));
                        for (const e of patEvents) {
                            events.push(new Event(
                                sliceSpan,
                                intersection,
                                e.value,
                                e.context
                            ));
                        }
                    }
                }
            }
        }
        
        return events;
    });
}

/**
 * Wedge - Create wedge pattern (gradually change between patterns)
 */
function wedge(cycles, pat1, pat2) {
    const n = typeof cycles === 'number' ? cycles : cycles.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const phase = (cycle % n) / n;
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                if (phase < 0.5) {
                    // More of pattern 1
                    const split = phase * 2;
                    const splitPoint = cycle + split;
                    
                    events.push(...pat1.query(new TimeSpan(cycle, splitPoint)));
                    events.push(...pat2.query(new TimeSpan(splitPoint, cycle + 1)));
                } else {
                    // More of pattern 2
                    const split = (phase - 0.5) * 2;
                    const splitPoint = cycle + (1 - split);
                    
                    events.push(...pat2.query(new TimeSpan(cycle, splitPoint)));
                    events.push(...pat1.query(new TimeSpan(splitPoint, cycle + 1)));
                }
            }
        }
        
        return events;
    });
}

/**
 * Chunk - Apply function to chunks of pattern
 */
function chunk(n, fn, pattern) {
    const chunkSize = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const chunkIndex = Math.floor(cycle / chunkSize);
            const chunkStart = chunkIndex * chunkSize;
            
            // Get pattern for this chunk
            const chunkPattern = compress(
                (cycle - chunkStart) / chunkSize,
                (cycle + 1 - chunkStart) / chunkSize,
                fn(slow(chunkSize, pattern))
            );
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                events.push(...chunkPattern.query(intersection));
            }
        }
        
        return events;
    });
}

/**
 * ChunksRev - Reverse chunks of pattern
 */
function chunksRev(n, pattern) {
    return chunk(n, rev, pattern);
}

/**
 * Splice - Splice patterns together at specific point
 */
function splice(point, pat1, pat2) {
    const p = typeof point === 'number' ? point : point.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const splitPoint = cycle + p;
            
            // First part from pat1
            const span1 = new TimeSpan(cycle, splitPoint);
            const int1 = span1.intersection(span);
            if (int1) {
                events.push(...pat1.query(int1));
            }
            
            // Second part from pat2
            const span2 = new TimeSpan(splitPoint, cycle + 1);
            const int2 = span2.intersection(span);
            if (int2) {
                events.push(...pat2.query(int2));
            }
        }
        
        return events;
    });
}

/**
 * Spin - Rotate pattern in a spinning motion
 */
function spin(n, pattern) {
    const cycles = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const rotation = (cycle % cycles) / cycles;
            const rotPattern = _rotL(rotation, pattern);
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                events.push(...rotPattern.query(intersection));
            }
        }
        
        return events;
    });
}

/**
 * Stripe - Create striped pattern
 */
function stripe(n, pattern) {
    const stripes = typeof n === 'number' ? n : n.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const stripeIndex = cycle % stripes;
            const stripeStart = cycle + stripeIndex / stripes;
            const stripeEnd = stripeStart + 1 / stripes;
            
            const stripeSpan = new TimeSpan(stripeStart, stripeEnd);
            const intersection = stripeSpan.intersection(span);
            
            if (intersection) {
                events.push(...pattern.query(intersection));
            }
        }
        
        return events;
    });
}

/**
 * Within - Apply function within time range
 */
function within(begin, end, fn, pattern) {
    const b = typeof begin === 'number' ? begin : begin.toFloat();
    const e = typeof end === 'number' ? end : end.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                const rangeStart = cycle + b;
                const rangeEnd = cycle + e;
                const rangeSpan = new TimeSpan(rangeStart, rangeEnd);
                
                const inRange = rangeSpan.intersection(intersection);
                const outRange1 = new TimeSpan(intersection.begin, Math.min(rangeStart, intersection.end));
                const outRange2 = new TimeSpan(Math.max(rangeEnd, intersection.begin), intersection.end);
                
                // Apply function within range
                if (inRange) {
                    events.push(...fn(pattern).query(inRange));
                }
                
                // Keep original outside range
                if (outRange1.begin.toFloat() < outRange1.end.toFloat()) {
                    events.push(...pattern.query(outRange1));
                }
                if (outRange2.begin.toFloat() < outRange2.end.toFloat()) {
                    events.push(...pattern.query(outRange2));
                }
            }
        }
        
        return events;
    });
}

/**
 * Withins - Apply multiple functions within time ranges
 */
function withins(ranges, pattern) {
    let result = pattern;
    for (const [begin, end, fn] of ranges) {
        result = within(begin, end, fn, result);
    }
    return result;
}

/**
 * Rot - Rotate pattern left (same as _rotL)
 */
function rot(amount, pattern) {
    return _rotL(amount, pattern);
}

/**
 * Scale - Apply musical scale to numeric pattern
 */
function scale(scaleName, pattern) {
    const scales = {
        major: [0, 2, 4, 5, 7, 9, 11],
        minor: [0, 2, 3, 5, 7, 8, 10],
        pentatonic: [0, 2, 4, 7, 9],
        chromatic: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        dorian: [0, 2, 3, 5, 7, 9, 10],
        phrygian: [0, 1, 3, 5, 7, 8, 10],
        lydian: [0, 2, 4, 6, 7, 9, 11],
        mixolydian: [0, 2, 4, 5, 7, 9, 10],
        aeolian: [0, 2, 3, 5, 7, 8, 10],
        locrian: [0, 1, 3, 5, 6, 8, 10]
    };
    
    const scaleNotes = scales[scaleName] || scales.major;
    
    return pattern.fmap(value => {
        if (typeof value === 'number') {
            const octave = Math.floor(value / scaleNotes.length);
            const index = Math.floor(value) % scaleNotes.length;
            return octave * 12 + scaleNotes[index];
        }
        return value;
    });
}

/**
 * ToScale - Map values to scale degrees
 */
function toScale(scaleNotes, pattern) {
    const notes = Array.isArray(scaleNotes) ? scaleNotes : [scaleNotes];
    
    return pattern.fmap(value => {
        if (typeof value === 'number') {
            const octave = Math.floor(value / notes.length);
            const index = Math.floor(value) % notes.length;
            return octave * 12 + notes[index];
        }
        return value;
    });
}

/**
 * Fmap - Map function over pattern values
 */
function fmap(fn, pattern) {
    return pattern.fmap(fn);
}

/**
 * While - Apply pattern while condition is true
 */
function whilePat(test, pattern) {
    return new Pattern((span) => {
        const events = pattern.query(span);
        return events.filter(event => test(event.value));
    });
}

/**
 * Whenmod - Apply function when cycle mod equals value
 */
function whenmod(divisor, remainder, fn, pattern) {
    const d = typeof divisor === 'number' ? divisor : divisor.toFloat();
    const r = typeof remainder === 'number' ? remainder : remainder.toFloat();
    
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                if (cycle % d === r) {
                    events.push(...fn(pattern).query(intersection));
                } else {
                    events.push(...pattern.query(intersection));
                }
            }
        }
        
        return events;
    });
}

/**
 * Trunc - Truncate pattern to n cycles
 */
function trunc(cycles, pattern) {
    const n = typeof cycles === 'number' ? cycles : cycles.toFloat();
    
    return new Pattern((span) => {
        const truncSpan = new TimeSpan(0, n);
        const querySpan = span.intersection(truncSpan);
        
        if (querySpan) {
            return pattern.query(querySpan);
        }
        return [];
    });
}

/**
 * Linger - Extend events to linger
 */
function linger(factor, pattern) {
    const f = typeof factor === 'number' ? factor : factor.toFloat();
    
    return new Pattern((span) => {
        const events = pattern.query(span);
        
        return events.map(event => {
            const newEnd = event.part.begin.add(event.part.duration.mul(f));
            return new Event(
                event.whole,
                new TimeSpan(event.part.begin, newEnd),
                event.value,
                event.context
            );
        });
    });
}

/**
 * Zoom2 - Alternative zoom implementation
 */
function zoom2(start, end, pattern) {
    const s = typeof start === 'number' ? start : start.toFloat();
    const e = typeof end === 'number' ? end : end.toFloat();
    
    return zoom(s, e, pattern);
}

/**
 * Discretise - Sample pattern at discrete intervals
 */
function discretise(n, pattern) {
    return segment(n, pattern);
}

/**
 * Smooth - Smooth pattern transitions
 */
function smooth(pattern) {
    return new Pattern((span) => {
        const events = pattern.query(span);
        const smoothed = [];
        
        for (let i = 0; i < events.length - 1; i++) {
            const curr = events[i];
            const next = events[i + 1];
            
            smoothed.push(curr);
            
            // Add interpolation event
            if (typeof curr.value === 'number' && typeof next.value === 'number') {
                const interpBegin = curr.part.end;
                const interpEnd = next.part.begin;
                const interpValue = (curr.value + next.value) / 2;
                
                if (interpBegin.toFloat() < interpEnd.toFloat()) {
                    smoothed.push(new Event(
                        new TimeSpan(interpBegin, interpEnd),
                        new TimeSpan(interpBegin, interpEnd),
                        interpValue,
                        { smooth: true }
                    ));
                }
            }
        }
        
        if (events.length > 0) {
            smoothed.push(events[events.length - 1]);
        }
        
        return smoothed;
    });
}

/**
 * Trigger - Trigger pattern on condition
 */
function trigger(resetPattern, pattern) {
    return new Pattern((span) => {
        const triggers = resetPattern.query(span);
        const events = [];
        
        for (const trigger of triggers) {
            const triggerTime = trigger.part.begin;
            const nextCycle = Math.ceil(triggerTime.toFloat());
            const triggerSpan = new TimeSpan(triggerTime, nextCycle);
            const intersection = triggerSpan.intersection(span);
            
            if (intersection) {
                // Shift pattern to start at trigger time
                const shifted = pattern.query(new TimeSpan(0, intersection.duration));
                for (const e of shifted) {
                    events.push(new Event(
                        e.whole.add(triggerTime),
                        e.part.add(triggerTime),
                        e.value,
                        e.context
                    ));
                }
            }
        }
        
        return events;
    });
}

/**
 * Qtrigger - Quantized trigger
 */
function qtrigger(resetPattern, pattern) {
    return new Pattern((span) => {
        const triggers = resetPattern.query(span);
        const events = [];
        
        for (const trigger of triggers) {
            const triggerTime = Math.floor(trigger.part.begin.toFloat());
            const nextCycle = triggerTime + 1;
            const triggerSpan = new TimeSpan(triggerTime, nextCycle);
            const intersection = triggerSpan.intersection(span);
            
            if (intersection) {
                events.push(...pattern.query(intersection));
            }
        }
        
        return events;
    });
}

/**
 * Reset - Reset pattern at trigger points
 */
function reset(resetPattern, pattern) {
    return trigger(resetPattern, pattern);
}

/**
 * Restart - Restart pattern at trigger points
 */
function restart(resetPattern, pattern) {
    return qtrigger(resetPattern, pattern);
}

/**
 * Ifp - If predicate then pattern1 else pattern2
 */
function ifp(predicate, thenPattern, elsePattern) {
    return new Pattern((span) => {
        const events = [];
        
        for (let cycle = Math.floor(span.begin.toFloat());
             cycle <= Math.ceil(span.end.toFloat());
             cycle++) {
            
            const cycleSpan = new TimeSpan(cycle, cycle + 1);
            const intersection = cycleSpan.intersection(span);
            
            if (intersection) {
                if (predicate(cycle)) {
                    events.push(...thenPattern.query(intersection));
                } else {
                    events.push(...elsePattern.query(intersection));
                }
            }
        }
        
        return events;
    });
}

/**
 * Compress2 - Alternative compress implementation  
 */
function compress2(start, end, pattern) {
    return compress(start, end, pattern);
}

/**
 * Expand - Expand pattern (opposite of compress)
 */
function expand(start, end, pattern) {
    const s = typeof start === 'number' ? start : start.toFloat();
    const e = typeof end === 'number' ? end : end.toFloat();
    
    if (s >= e) return silence();
    const duration = e - s;
    
    return new Pattern((span) => {
        // Map query span to expanded range
        const expandedBegin = span.begin.sub(s).div(duration);
        const expandedEnd = span.end.sub(s).div(duration);
        
        return pattern.query(new TimeSpan(expandedBegin, expandedEnd))
            .map(event => new Event(
                event.whole.mul(duration).add(s),
                event.part.mul(duration).add(s),
                event.value,
                event.context
            ));
    });
}

/**
 * Append - Append pattern after another
 */
function append(pat1, pat2) {
    return slowcat(pat1, pat2);
}

/**
 * Prepend - Prepend pattern before another
 */
function prepend(pat1, pat2) {
    return slowcat(pat1, pat2);
}

/**
 * Scan - Scan/accumulate pattern values
 */
function scan(fn, initial, pattern) {
    return new Pattern((span) => {
        const events = pattern.query(span);
        let acc = initial;
        
        return events.map(event => {
            acc = fn(acc, event.value);
            return new Event(
                event.whole,
                event.part,
                acc,
                event.context
            );
        });
    });
}

/**
 * Unfold - Unfold/generate pattern from seed
 */
function unfold(fn, seed, n) {
    const count = typeof n === 'number' ? n : n.toFloat();
    const values = [];
    let current = seed;
    
    for (let i = 0; i < count; i++) {
        values.push(current);
        current = fn(current);
    }
    
    return sequence(...values.map(pure));
}

/**
 * Gain - Set gain/volume metadata
 */
function gain(amount, pattern) {
    const g = typeof amount === 'number' ? amount : amount.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, gain: g };
        }
        return { value, gain: g };
    });
}

/**
 * Legato - Set legato/sustain metadata
 */
function legato(amount, pattern) {
    const l = typeof amount === 'number' ? amount : amount.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, legato: l };
        }
        return { value, legato: l };
    });
}

/**
 * N - Create pattern from number (sample index)
 */
function n(index) {
    return pure({ n: index });
}

/**
 * Note - Create pattern from note number
 */
function note(noteNum) {
    return pure({ note: noteNum });
}

/**
 * Speed - Set playback speed metadata
 */
function speed(factor, pattern) {
    const s = typeof factor === 'number' ? factor : factor.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, speed: s };
        }
        return { value, speed: s };
    });
}

/**
 * Unit - Set time unit metadata
 */
function unit(u, pattern) {
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, unit: u };
        }
        return { value, unit: u };
    });
}

/**
 * Begin - Set begin position metadata
 */
function begin(pos, pattern) {
    const p = typeof pos === 'number' ? pos : pos.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, begin: p };
        }
        return { value, begin: p };
    });
}

/**
 * End - Set end position metadata
 */
function end(pos, pattern) {
    const p = typeof pos === 'number' ? pos : pos.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, end: p };
        }
        return { value, end: p };
    });
}

/**
 * Pan - Set stereo pan metadata
 */
function pan(amount, pattern) {
    const p = typeof amount === 'number' ? amount : amount.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, pan: p };
        }
        return { value, pan: p };
    });
}

/**
 * Shape - Set envelope shape metadata
 */
function shape(amount, pattern) {
    const s = typeof amount === 'number' ? amount : amount.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, shape: s };
        }
        return { value, shape: s };
    });
}

/**
 * Crush - Set bit crush metadata
 */
function crush(bits, pattern) {
    const b = typeof bits === 'number' ? bits : bits.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, crush: b };
        }
        return { value, crush: b };
    });
}

/**
 * Coarse - Set coarse pitch metadata
 */
function coarse(semitones, pattern) {
    const c = typeof semitones === 'number' ? semitones : semitones.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, coarse: c };
        }
        return { value, coarse: c };
    });
}

/**
 * Delay - Set delay send metadata
 */
function delay(amount, pattern) {
    const d = typeof amount === 'number' ? amount : amount.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, delay: d };
        }
        return { value, delay: d };
    });
}

/**
 * Delaytime - Set delay time metadata
 */
function delaytime(time, pattern) {
    const t = typeof time === 'number' ? time : time.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, delaytime: t };
        }
        return { value, delaytime: t };
    });
}

/**
 * Delayfeedback - Set delay feedback metadata
 */
function delayfeedback(fb, pattern) {
    const f = typeof fb === 'number' ? fb : fb.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, delayfeedback: f };
        }
        return { value, delayfeedback: f };
    });
}

/**
 * Vowel - Set vowel filter metadata
 */
function vowel(v, pattern) {
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, vowel: v };
        }
        return { value, vowel: v };
    });
}

/**
 * Room - Set room/reverb size metadata
 */
function room(size, pattern) {
    const r = typeof size === 'number' ? size : size.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, room: r };
        }
        return { value, room: r };
    });
}

/**
 * Size - Set reverb size metadata (alias for room)
 */
function size(s, pattern) {
    return room(s, pattern);
}

/**
 * Orbit - Set orbit/channel metadata
 */
function orbit(channel, pattern) {
    const o = typeof channel === 'number' ? channel : channel.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, orbit: o };
        }
        return { value, orbit: o };
    });
}

/**
 * Cutoff - Set filter cutoff metadata
 */
function cutoff(freq, pattern) {
    const c = typeof freq === 'number' ? freq : freq.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, cutoff: c };
        }
        return { value, cutoff: c };
    });
}

/**
 * Resonance - Set filter resonance metadata
 */
function resonance(amount, pattern) {
    const r = typeof amount === 'number' ? amount : amount.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, resonance: r };
        }
        return { value, resonance: r };
    });
}

/**
 * Attack - Set envelope attack metadata
 */
function attack(time, pattern) {
    const a = typeof time === 'number' ? time : time.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, attack: a };
        }
        return { value, attack: a };
    });
}

/**
 * Release - Set envelope release metadata
 */
function release(time, pattern) {
    const r = typeof time === 'number' ? time : time.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, release: r };
        }
        return { value, release: r };
    });
}

/**
 * Hold - Set envelope hold metadata
 */
function hold(time, pattern) {
    const h = typeof time === 'number' ? time : time.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, hold: h };
        }
        return { value, hold: h };
    });
}

/**
 * Bandf - Set bandpass filter frequency metadata
 */
function bandf(freq, pattern) {
    const f = typeof freq === 'number' ? freq : freq.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, bandf: f };
        }
        return { value, bandf: f };
    });
}

/**
 * Bandq - Set bandpass filter Q metadata
 */
function bandq(q, pattern) {
    const bq = typeof q === 'number' ? q : q.toFloat();
    
    return pattern.fmap(value => {
        if (typeof value === 'object' && value !== null) {
            return { ...value, bandq: bq };
        }
        return { value, bandq: bq };
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
    euclidLegato,
    
    // Pattern combination
    jux,
    juxBy,
    superimpose,
    layer,
    off,
    echo,
    stut,
    
    // Filtering & masking
    when,
    mask,
    struct,
    filter,
    
    // Math operations
    add,
    sub,
    mul,
    div,
    mod,
    range,
    
    // Additional patterns
    sequence,
    polymeter,
    polyrhythm,
    
    // More pattern structure
    firstOf,
    lastOf,
    brak,
    press,
    hurry,
    bite,
    striate,
    inhabit,
    
    // Arpeggiation
    arp,
    arpWith,
    
    // More math
    rangex,
    
    // Step sequencing
    fit,
    take,
    drop,
    run,
    steps,
    
    // Cycle-based randomness
    someCycles,
    someCyclesBy,
    
    // Timing manipulation
    swing,
    swingBy,
    
    // Grid patterns
    grid,
    place,
    
    // Pattern generation
    binary,
    ascii,
    
    // Bipolar signals
    saw2,
    sine2,
    square2,
    tri2,
    
    // Pattern weaving
    weave,
    wedge,
    
    // Chunking
    chunk,
    chunksRev,
    
    // Splicing
    splice,
    
    // Rotation & movement
    spin,
    stripe,
    rot,
    
    // Time ranges
    within,
    withins,
    
    // Musical scales
    scale,
    toScale,
    
    // Functional
    fmap,
    while: whilePat,
    whenmod,
    
    // Pattern control
    trunc,
    linger,
    zoom2,
    discretise,
    
    // Smoothing & transitions
    smooth,
    
    // Triggers & resets
    trigger,
    qtrigger,
    reset,
    restart,
    
    // Conditional patterns
    ifp,
    
    // Additional compression
    compress2,
    expand,
    
    // Pattern combination
    append,
    prepend,
    
    // Functional patterns
    scan,
    unfold,
    
    // Audio metadata operators
    gain,
    legato,
    n,
    note,
    speed,
    unit,
    begin,
    end,
    pan,
    shape,
    crush,
    coarse,
    delay,
    delaytime,
    delayfeedback,
    vowel,
    room,
    size,
    orbit,
    cutoff,
    resonance,
    attack,
    release,
    hold,
    bandf,
    bandq
};
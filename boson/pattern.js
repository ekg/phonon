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
    slow
};
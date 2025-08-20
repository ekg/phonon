/**
 * PROPER Mini-notation parser for TidalCycles/Strudel syntax
 */

const {
    Pattern,
    pure,
    silence,
    stack,
    cat,
    fastcat,
    slowcat,
    euclid,
    fast,
    slow,
    rev,
    every,
    degrade,
    sometimes,
    range,
    run,
    choose
} = require('./pattern');

class MiniParser {
    constructor() {
        this.pos = 0;
        this.input = '';
    }
    
    parse(input) {
        this.input = input.trim();
        this.pos = 0;
        
        // Remove quotes if present
        if ((this.input.startsWith('"') && this.input.endsWith('"')) ||
            (this.input.startsWith("'") && this.input.endsWith("'"))) {
            this.input = this.input.slice(1, -1);
        }
        
        return this.parsePattern();
    }
    
    parsePattern() {
        // Check for stack (comma-separated at top level)
        if (this.hasTopLevelComma()) {
            return this.parseStack();
        }
        return this.parseSequence();
    }
    
    hasTopLevelComma() {
        let depth = 0;
        for (let i = 0; i < this.input.length; i++) {
            const char = this.input[i];
            if (char === '[' || char === '<') depth++;
            else if (char === ']' || char === '>') depth--;
            else if (char === ',' && depth === 0) return true;
        }
        return false;
    }
    
    parseStack() {
        const parts = this.splitTopLevel(',');
        const patterns = parts.map(part => {
            const parser = new MiniParser();
            return parser.parse(part.trim());
        });
        return stack(...patterns);
    }
    
    parseSequence() {
        const elements = [];
        
        while (this.pos < this.input.length) {
            this.skipWhitespace();
            if (this.pos >= this.input.length) break;
            
            const element = this.parseElement();
            if (element) {
                elements.push(element);
            }
        }
        
        if (elements.length === 0) return silence();
        if (elements.length === 1) return elements[0];
        return fastcat(...elements);
    }
    
    parseElement() {
        this.skipWhitespace();
        
        const char = this.input[this.pos];
        
        // Group [bd cp] - multiple in one step
        if (char === '[') {
            return this.parseGroup();
        }
        
        // Alternation <bd sn cp> - cycles through options
        if (char === '<') {
            return this.parseAlternation();
        }
        
        // Rest
        if (char === '~' || char === '.') {
            this.pos++;
            return silence();
        }
        
        // Sample or pattern
        return this.parseSample();
    }
    
    parseGroup() {
        this.pos++; // Skip [
        const groupContent = this.readUntil(']');
        this.pos++; // Skip ]
        
        // Parse the group content
        const parser = new MiniParser();
        const groupPattern = parser.parse(groupContent);
        
        // Check for operators after the group
        return this.parseOperators(groupPattern);
    }
    
    parseAlternation() {
        this.pos++; // Skip <
        const altContent = this.readUntil('>');
        this.pos++; // Skip >
        
        // Split by spaces to get alternatives
        const alts = altContent.trim().split(/\s+/);
        
        // Create a pattern that alternates each cycle
        // This is a simplified version - proper implementation would use `slow`
        return slowcat(...alts.map(a => {
            if (a === '~') return silence();
            return pure({ type: 'sample', name: a });
        }));
    }
    
    parseSample() {
        let token = '';
        
        // Read the sample name
        while (this.pos < this.input.length) {
            const char = this.input[this.pos];
            if (char === ' ' || char === '[' || char === ']' || 
                char === '<' || char === '>' || char === ',' ||
                char === '(' || char === '*' || char === '/' ||
                char === ':' || char === '!') {
                break;
            }
            token += char;
            this.pos++;
        }
        
        if (!token) return null;
        
        // Handle rest
        if (token === '~' || token === '.') {
            return silence();
        }
        
        // Create base pattern
        let pattern = pure({ type: 'sample', name: token });
        
        // Check for operators
        return this.parseOperators(pattern);
    }
    
    parseOperators(pattern) {
        while (this.pos < this.input.length) {
            const char = this.input[this.pos];
            
            // Euclidean rhythm: bd(3,8) or bd(3,8,1)
            if (char === '(') {
                this.pos++;
                const params = this.readUntil(')').split(',').map(p => parseInt(p.trim()));
                this.pos++;
                
                if (params.length >= 2) {
                    const [k, n, rot = 0] = params;
                    // For euclidean, we need to extract the sample name and apply euclid
                    const sampleName = pattern.query(new TimeSpan(0, 1))[0]?.value?.name;
                    if (sampleName) {
                        pattern = euclid(k, n, rot).fmap(v => 
                            v !== null ? { type: 'sample', name: sampleName } : null
                        );
                    }
                }
            }
            // Repetition: bd*4
            else if (char === '*') {
                this.pos++;
                const num = this.readNumber();
                pattern = fast(num, pattern);
            }
            // Slow down: bd/2
            else if (char === '/') {
                this.pos++;
                const num = this.readNumber();
                pattern = slow(num, pattern);
            }
            // Duration: bd:0.5 (we'll ignore for now as it needs different handling)
            else if (char === ':') {
                this.pos++;
                this.readNumber(); // Just consume it
            }
            // Degrade: bd!
            else if (char === '!') {
                this.pos++;
                pattern = degrade(pattern);
            }
            // Reverse: bd@
            else if (char === '@') {
                this.pos++;
                pattern = rev(pattern);
            }
            else {
                break;
            }
        }
        
        return pattern;
    }
    
    readUntil(endChar) {
        let result = '';
        let depth = 0;
        
        while (this.pos < this.input.length) {
            const char = this.input[this.pos];
            
            if (char === '[' || char === '<') depth++;
            else if (char === ']' || char === '>') depth--;
            
            if (char === endChar && depth === 0) {
                break;
            }
            
            result += char;
            this.pos++;
        }
        
        return result;
    }
    
    readNumber() {
        let num = '';
        while (this.pos < this.input.length) {
            const char = this.input[this.pos];
            if (char >= '0' && char <= '9' || char === '.') {
                num += char;
                this.pos++;
            } else {
                break;
            }
        }
        return parseFloat(num) || 1;
    }
    
    skipWhitespace() {
        while (this.pos < this.input.length && this.input[this.pos] === ' ') {
            this.pos++;
        }
    }
    
    splitTopLevel(delimiter) {
        const parts = [];
        let current = '';
        let depth = 0;
        
        for (let i = 0; i < this.input.length; i++) {
            const char = this.input[i];
            
            if (char === '[' || char === '<') depth++;
            else if (char === ']' || char === '>') depth--;
            
            if (char === delimiter && depth === 0) {
                parts.push(current);
                current = '';
            } else {
                current += char;
            }
        }
        
        if (current) parts.push(current);
        return parts;
    }
}

// Need TimeSpan for querying
class TimeSpan {
    constructor(begin, end) {
        this.begin = typeof begin === 'number' ? begin : begin.toFloat();
        this.end = typeof end === 'number' ? end : end.toFloat();
    }
}

module.exports = MiniParser;
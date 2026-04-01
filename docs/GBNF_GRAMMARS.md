# GBNF Grammars for Elma JSON Output

## Purpose
Force model to produce valid JSON at token generation level, not through prompts.

## Grammar Files Location
`config/{model}/grammars/`

---

## 1. Orchestrator Grammar (Program)

**File:** `program_grammar.gbnf`

```gbnf
root ::= program
program ::= "{" ws "\"objective\"" ws ":" ws string ws "," ws "\"steps\"" ws ":" ws "[" ws (step (ws "," ws step)*)? ws "]" ws "}"

step ::= shell_step | reply_step | plan_step | masterplan_step | select_step | decide_step | summarize_step | edit_step

shell_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"shell\"" ws "," ws "\"cmd\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

reply_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"reply\"" ws "," ws "\"instructions\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

plan_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"plan\"" ws "," ws "\"goal\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

masterplan_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"masterplan\"" ws "," ws "\"goal\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

select_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"select\"" ws "," ws "\"instructions\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

decide_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"decide\"" ws "," ws "\"prompt\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

summarize_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"summarize\"" ws "," ws "\"text\"" ws ":" ws string ws "," ws "\"instructions\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

edit_step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws "\"edit\"" ws "," ws "\"path\"" ws ":" ws string ws "," ws "\"operation\"" ws ":" ws edit_op ws "," ws "\"content\"" ws ":" ws string ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws "}"

edit_op ::= "\"create\"" | "\"update\"" | "\"delete\""

string ::= "\"" char* "\""
char ::= [^"\\\r\n] | "\\" escape
escape ::= ["\\bfnrt]
ws ::= [ \t\n\r]*
```

---

## 2. Critic Verdict Grammar

**File:** `critic_verdict_grammar.gbnf`

```gbnf
root ::= verdict
verdict ::= "{" ws "\"status\"" ws ":" ws status ws "," ws "\"reason\"" ws ":" ws string (ws "," ws "\"program\"" ws ":" ws (program | "null"))? ws "}"

status ::= "\"ok\"" | "\"retry\""
program ::= root  # Reference to program grammar
string ::= "\"" char* "\""
char ::= [^"\\\r\n] | "\\" escape
escape ::= ["\\bfnrt]
ws ::= [ \t\n\r]*
```

---

## 3. Risk Review Verdict Grammar

**File:** `risk_verdict_grammar.gbnf`

```gbnf
root ::= verdict
verdict ::= "{" ws "\"status\"" ws ":" ws status ws "," ws "\"reason\"" ws ":" ws string ws "}"

status ::= "\"ok\"" | "\"caution\""
string ::= "\"" char* "\""
char ::= [^"\\\r\n] | "\\" escape
escape ::= ["\\bfnrt]
ws ::= [ \t\n\r]*
```

---

## 4. Outcome Verification Verdict Grammar

**File:** `outcome_verdict_grammar.gbnf`

```gbnf
root ::= verdict
verdict ::= "{" ws "\"status\"" ws ":" ws status ws "," ws "\"reason\"" ws ":" ws string ws "}"

status ::= "\"ok\"" | "\"retry\""
string ::= "\"" char* "\""
char ::= [^"\\\r\n] | "\\" escape
escape ::= ["\\bfnrt]
ws ::= [ \t\n\r]*
```

---

## 5. Self-Question Verdict Grammar

**File:** `self_question_grammar.gbnf`

```gbnf
root ::= verdict
verdict ::= "{" ws "\"method\"" ws ":" ws method ws "," ws "\"reason\"" ws ":" ws string ws "," ws "\"internal_command\"" ws ":" ws (string | "null") ws "}"

method ::= "\"SHELL\"" | "\"INTERNAL\""
string ::= "\"" char* "\""
char ::= [^"\\\r\n] | "\\" escape
escape ::= ["\\bfnrt]
ws ::= [ \t\n\r]*
```

---

## 6. Refinement Grammar (Program)

**File:** Same as orchestrator - `program_grammar.gbnf`

---

## 7. Sufficiency Verdict Grammar

**File:** `sufficiency_verdict_grammar.gbnf`

```gbnf
root ::= verdict
verdict ::= "{" ws "\"status\"" ws ":" ws status ws "," ws "\"reason\"" ws ":" ws string (ws "," ws "\"program\"" ws ":" ws (program | "null"))? ws "}"

status ::= "\"ok\"" | "\"retry\""
program ::= root  # Reference to program grammar
string ::= "\"" char* "\""
char ::= [^"\\\r\n] | "\\" escape
escape ::= ["\\bfnrt]
ws ::= [ \t\n\r]*
```

---

## Usage in Code

```rust
// Load grammar file
let grammar_path = model_cfg_dir.join("grammars/program_grammar.gbnf");
let grammar = std::fs::read_to_string(&grammar_path).ok();

// Pass to ChatCompletionRequest
let req = ChatCompletionRequest {
    // ... other fields ...
    grammar,  // Option<String>
};
```

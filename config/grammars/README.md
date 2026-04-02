# GBNF Grammars for JSON Output Enforcement

## Purpose

These GBNF (Grammar Backus-Naur Form) files enforce 100% valid JSON output from llama.cpp model.

## Usage

Add to profile TOML:
```toml
grammar_path = "grammars/router_choice_1of5.json.gbnf"
```

The grammar is loaded and injected into ChatCompletionRequest.

## Grammar Files

| File | Purpose | Choices |
|------|---------|---------|
| `router_choice_1of5.json.gbnf` | Route classification | CHAT, INVESTIGATE, SHELL, PLAN, MASTERPLAN |
| `speech_act_choice_1of3.json.gbnf` | Speech act classification | CHAT, INQUIRE, INSTRUCT |
| `mode_router_choice_1of4.json.gbnf` | Mode selection | INSPECT, EXECUTE, PLAN, MASTERPLAN |
| `complexity_choice_1of4.json.gbnf` | Complexity assessment | DIRECT, INVESTIGATE, MULTISTEP, OPEN_ENDED |

## Testing

Test grammar with curl:
```bash
curl -X POST http://192.168.1.186:8080/completion \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Classify: List files",
    "n_predict": 100,
    "temperature": 0.1,
    "grammar": "root ::= \"{\\\"choice\\\":\\\"TEST\"}"
  }'
```

Expected: Model can ONLY output valid JSON matching grammar.

## Grammar Syntax

```bnf
root ::= "{" ws "\"choice\":" ws string ws "," ws "\"label\":" ws string "}"
string ::= "\"" [a-zA-Z0-9_ ]* "\""
ws ::= [ \t\n]*
```

- `::=` defines a rule
- `|` denotes choice (OR)
- `*` denotes zero-or-more
- Quoted strings are literals
- Whitespace rules prevent accidental whitespace in output

## Troubleshooting

**Problem:** Grammar not enforced
**Solution:** Verify llama.cpp compiled with LLAMA_GRAMMAR flag

**Problem:** Model outputs invalid JSON
**Solution:** Check grammar file syntax, ensure proper escaping

**Problem:** Latency increased
**Solution:** Grammar constraints add 5-10% overhead (acceptable)

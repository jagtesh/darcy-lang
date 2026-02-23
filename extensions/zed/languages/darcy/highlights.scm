["(" ")" "[" "]" "{" "}"] @punctuation.bracket

(number) @number
(character) @constant.builtin
(boolean) @constant.builtin
((keyword) @constant
 (#match? @constant "^:"))
((symbol) @constant
 (#match? @constant "^:"))
((symbol) @function
 (#match? @function "^[^:]+:[^:]+$"))
((keyword) @function
 (#match? @function "^[^:]+:[^:]+$"))
((symbol) @variable
 (#match? @variable "^[^:]"))

(string) @string

(escape_sequence) @escape

(list
  .
  (symbol) @function
  (#match? @function "^[^:]+$"))

((symbol) @operator
 (#match? @operator "^(\\+|-|\\*|/|=|>|<|>=|<=|mod|&|\\||->|->>)$"))

(list
  .
  (symbol) @keyword
  (#match? @keyword
   "^(def|defn|defin|defmacro|defrecord|defenum|extern|export|if|when|cond|case|do|let|let!|fn|loop|while|for|break|continue|require|quote|syntax-quote|unquote|unquote-splicing|with-meta|call|range|range-incl|and|or|true|false|nil)$"
   ))

[(comment)
 (block_comment)
 (directive)] @comment

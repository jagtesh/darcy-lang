["(" ")" "[" "]" "{" "}"] @punctuation.bracket

(number) @number
(character) @constant.builtin
(boolean) @constant.builtin
(keyword) @keyword
((symbol) @keyword
 (#match? @keyword "^:"))
((symbol) @variable
 (#match? @variable "^[^:]"))

(string) @string

(escape_sequence) @escape

(list
  .
  (symbol) @function)

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

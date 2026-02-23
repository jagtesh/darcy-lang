["(" ")" "[" "]" "{" "}"] @punctuation.bracket

(number) @number
(boolean) @constant.builtin
(string) @string
(comment) @comment
(block_comment) @comment
(directive) @comment

; symbols and keywords
(symbol) @variable

((symbol) @keyword
 (#match? @keyword "^(def|defn|defin|defmacro|defrecord|defenum|extern|export|if|when|cond|case|do|let|let!|fn|loop|while|for|break|continue|require|quote|syntax-quote|unquote|unquote-splicing|with-meta|call|range|range-incl|true|false|nil)$"))

((symbol) @operator
 (#match? @operator "^(\+|-|\*|/|mod|=|>|<|>=|<=|&|\||->|->>)$"))

; callable form head
(list
  .
  (symbol) @function)

; namespaced call heads
(list
  .
  (symbol) @function.method
  (#match? @function.method "^[a-z][a-z0-9._-]*/[a-zA-Z0-9_?!*+=<>$-]+$"))

; :keyword literals
((keyword) @property)

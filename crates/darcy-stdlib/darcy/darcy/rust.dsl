(defmacro defextern [name params ret rust-name]
  `(extern ~rust-name (defn ~name ~params ~ret)))

(defmacro defextern-record [name rust-name fields]
  `(extern ~rust-name (defrecord ~name ~@fields)))

(defmacro defextern-enum [name rust-name variants]
  `(extern ~rust-name (defenum ~name ~@variants)))

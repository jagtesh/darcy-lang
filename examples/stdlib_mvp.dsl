(require [darcy.fmt :as fmt])
(require [darcy.io :as io])
(require [darcy.math])
(require [darcy.vec])
(require [darcy.string])
(require [darcy.option :as opt])
(require [darcy.result :as res])
(require [darcy.hash-map :as hmap])
(require [darcy.btree-map :as bmap])

(defrecord demo
  [greeting:string]
  [str-len:usize]
  [str-empty:bool]
  [num:i32]
  [abs:i32]
  [min:i32]
  [len:usize]
  [empty:bool]
  [opt:option<i32>]
  [opt-some:bool]
  [opt-none:bool]
  [opt-unwrap:i32]
  [opt-unwrap-or:i32]
  [res:result<i32,string>]
  [res-ok:bool]
  [res-err:bool]
  [res-unwrap:i32]
  [res-unwrap-or:i32]
  [hmap:hash-map<string,i32>]
  [hmap-len:usize]
  [hmap-contains:bool]
  [hmap-get:option<i32>]
  [hmap-insert:hash-map<string,i32>]
  [hmap-remove:hash-map<string,i32>]
  [bmap:btree-map<string,i32>]
  [bmap-len:usize]
  [bmap-contains:bool]
  [bmap-get:option<i32>]
  [bmap-insert:btree-map<string,i32>]
  [bmap-remove:btree-map<string,i32>])

(defn main []
  (io/dbg
    (demo
      (darcy.string/join
        (darcy.string/split
          (darcy.string/trim "  hello world  ")
          " ")
        "|")
      (darcy.string/len "darcy")
      (darcy.string/is-empty "")
      (darcy.math/clamp (darcy.math/max 3 5) 0 10)
      (darcy.math/abs -42)
      (darcy.math/min -2 7)
      (darcy.vec/len [1 2 3])
      (darcy.vec/is-empty (vec<i32>))
      (opt/some 1)
      (opt/is-some (opt/some 1))
      (opt/is-none (opt/none))
      (opt/unwrap (opt/some 9))
      (opt/unwrap-or (opt/none) 5)
      (res/ok 1)
      (res/is-ok (res/ok 1))
      (res/is-err (res/err "oops"))
      (res/unwrap (res/ok 2))
      (res/unwrap-or (res/err "oops") 7)
      (hmap/new [:a 1] [:b 2])
      (hmap/len (hmap/new [:a 1] [:b 2]))
      (hmap/contains (hmap/new [:a 1]) :a)
      (hmap/get (hmap/new [:a 1]) :a)
      (hmap/insert (hmap/new [:a 1]) :b 2)
      (hmap/remove (hmap/new [:a 1] [:b 2]) :a)
      (bmap/new [:x 9] [:y 8])
      (bmap/len (bmap/new [:x 9] [:y 8]))
      (bmap/contains (bmap/new [:x 9]) :x)
      (bmap/get (bmap/new [:x 9]) :x)
      (bmap/insert (bmap/new [:x 9]) :y 8)
      (bmap/remove (bmap/new [:x 9] [:y 8]) :x))))

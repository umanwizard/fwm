(setvbuf (current-output-port) 'line)
(set! *random-state* (random-state-from-platform))
(define at-point
  (lambda fs
    (lambda (wm)
      (let ([pt (fwm-get-point wm)])
	(for-each (lambda (f) (f wm pt)) fs)))))

;; https://stackoverflow.com/questions/26539585/how-to-display-multiple-parameters-in-r5rs-scheme
(define (insert-between v xs)
  (cond ((null? xs) xs)
        ((null? (cdr xs)) xs)
        (else (cons (car xs)
                    (cons v (insert-between v (cdr xs)))))))
(define (display-all . vs)
  (for-each display (insert-between " " vs)))

(define (println . vs)
  (apply display-all vs)
  (newline))

(define terminal "sakura")
(define exec
  (lambda (cmd)
    (system (string-append cmd "&"))))

(system "vmware-user-suid-wrapper")
(system "xmodmap ~/.Xmodmap")
(exec "xscreensaver -no-splash")
(exec "xcompmgr")


(use-modules (ice-9 ftw))

(define random-wallpaper
  (lambda ()
    (let* ([wp-dir "/home/brennan/wallpapers/"]
           [wps (scandir wp-dir (lambda (f) (or (string-suffix? ".jpg" f) (string-suffix? ".png" f))))]
           [idx (random (length wps))]
           [entry (list-ref wps idx)])
      (string-append wp-dir entry))))

(define (clear-wallpaper)
  (set! wall-past '())
  (set! wall-future (list (random-wallpaper)))
  (set! wall-cur #f))

(define wall-past '())

(define wall-future (list (random-wallpaper)))

(define wall-cur #f)

(define wall-back
  (lambda ()
    (if (equal? wall-past '())
	#f
	(let ([new (car wall-past)]
	      [old wall-cur])
	  (set! wall-past (cdr wall-past))
	  (set! wall-cur new)
      (if old
	    (set! wall-future (cons old wall-future)))
	  new))))

(define wall-fwd
  (lambda ()
    (if (equal? wall-future '())
	#f
	(let ([new (car wall-future)]
	      [old wall-cur])
	  (set! wall-future (cdr wall-future))
	  (set! wall-cur new)
	  (if old 
	      (set! wall-past (cons old wall-past)))
	  new))))

(import (rnrs base (6)))

(define set-wallpaper-killing-future
  (lambda ()
    (set! wall-future '())
    (set! wall-cur #f)
    (set-wallpaper)))

(define do-set-wp
  (lambda (f)
    (system* "feh" "--bg-max" f)))

(define set-wallpaper
  (lambda ()
    (if (equal? wall-future '())
	(set! wall-future (list (random-wallpaper))))
    (let ([wp (wall-fwd)])
      (assert wp)
      (do-set-wp wp))))

(set-wallpaper)

(define rust-option-to-scheme
  (lambda (op)
    (cond
     [(eq? op '()) #f]
     [else (car op)])))

(define item-for-cursor
  (lambda (wm cursor)
    (cond
     [(eq? (car cursor) 'Split)
      (assq-ref (cdr cursor) 'item)]
     [(eq? (car cursor) 'Into)
      (let ([container (assq-ref (cdr cursor) 'container)]
            [index (assq-ref (cdr cursor) 'index)])
	(println container index)
           (rust-option-to-scheme (fwm-nth-child wm container index)))])))

(define get-cursor-or-default
  (lambda (wm)
    (let ([cur (fwm-get-cursor wm)])
      (cond
       [(eq? cur '())
         (let* ([parent-slot (fwm-child-location wm (fwm-get-point wm))]
                [ctr (assq-ref parent-slot 'container)]
                [index (assq-ref parent-slot 'index)])
               (cons 'Into
                 (list
                   (cons 'container ctr)
                   (cons 'index index)
                 )))]
       [else (car cur)]))))

(define make-split-cursor
  (lambda (item dir)
    (cons 'Split
      (list (cons 'item item) (cons 'direction dir)))))

(define set-split
  (lambda (wm dir)
    (let* ([cur (get-cursor-or-default wm)]
           [item (item-for-cursor wm cur)])
      (if item (fwm-set-cursor wm (list (make-split-cursor item dir)))))))
      
(define place-layout-slot
  (lambda (wm)
    (let ([cursor (rust-option-to-scheme (fwm-get-cursor wm))])
      (if cursor (cons 'Move cursor)
	  (let* ([point (fwm-get-point wm)]
		 [container (fwm-nearest-container wm point)]
		 [n_ctr_children (fwm-n-children wm container)])
	    (fwm-make-cursor-into container n_ctr_children))))))

(define copy-ss
  (lambda ()
    (exec "scrot -s -f '/tmp/%F_%T_$wx$h.png' -e 'xclip -selection clipboard -target image/png -i $f'")))

(define (foreach-leaf f wm pt)
  (define (is-leaf pt)
    (eq? (car pt) 'Window))
  (let* ([descendants (fwm-all-descendants wm pt)]
	 [leaves (filter is-leaf descendants)])
    (println "Foreach leaves: " leaves)
    (for-each
     f
     leaves)
    ))

(define (request-kill-all wm pt)
  (foreach-leaf (lambda (leaf)
		  (fwm-request-kill-client-at wm (cdr leaf)))
		wm pt))

(define (protect-all wm pt)
  (foreach-leaf (lambda (leaf)
		  (set! protected-points (cons leaf protected-points)))
        wm pt))

(define (unprotect-all wm pt)
  (foreach-leaf (lambda (leaf)
          (set! protected-points (delete leaf protected-points)))
        wm pt))


(define (adjust-length wm pt f)
  (let ([length (fwm-get-length wm pt)])
    (if length
	(fwm-set-length wm pt (f length)))))

(define (increase-length wm pt)
  (adjust-length wm pt (lambda (x) (+ x 50))))

(define (decrease-length wm pt)
  (adjust-length wm pt (lambda (x) (- x 50))))

(use-modules (ice-9 pretty-print))
(define bindings
  (let ([mod "mod3"])
    (list
     (cons (fwm-parse-key-combo (string-append mod "+h")) (lambda (x) (fwm-navigate x '(Planar . Left))))
     (cons (fwm-parse-key-combo (string-append mod "+j")) (lambda (x) (fwm-navigate x '(Planar . Down))))
     (cons (fwm-parse-key-combo (string-append mod "+k")) (lambda (x) (fwm-navigate x '(Planar . Up))))
     (cons (fwm-parse-key-combo (string-append mod "+l")) (lambda (x) (fwm-navigate x '(Planar . Right))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+apostrophe")) (at-point fwm-kill-item-at))
     (cons (fwm-parse-key-combo (string-append mod "+apostrophe")) (at-point fwm-kill-client-at))
     (cons (fwm-parse-key-combo (string-append mod "+shift+h")) (lambda (x) (fwm-cursor x '(Planar . Left))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+j")) (lambda (x) (fwm-cursor x '(Planar . Down))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+k")) (lambda (x) (fwm-cursor x '(Planar . Up))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+l")) (lambda (x) (fwm-cursor x '(Planar . Right))))
     (cons (fwm-parse-key-combo (string-append mod "+a")) (lambda (x) (fwm-navigate x 'Parent)))
     (cons (fwm-parse-key-combo (string-append mod "+d")) (lambda (x) (fwm-navigate x 'Child)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+a")) (lambda (x) (fwm-cursor x 'Parent)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+d")) (lambda (x) (fwm-cursor x 'Child)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+period")) (lambda (x) (quit)))
     (cons (fwm-parse-key-combo (string-append mod "+p")) (at-point protect-all))
     (cons (fwm-parse-key-combo (string-append mod "+shift+p")) (at-point unprotect-all))
     (cons (fwm-parse-key-combo (string-append mod "+v")) (lambda (wm) (set-split wm 'Down)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+v")) (lambda (wm) (set-split wm 'Up)))
     (cons (fwm-parse-key-combo (string-append mod "+m")) (lambda (wm) (set-split wm 'Right)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+m")) (lambda (wm) (set-split wm 'Left)))
     (cons (fwm-parse-key-combo (string-append mod "+Escape")) (lambda (wm) (fwm-set-cursor wm '())))
     (cons (fwm-parse-key-combo (string-append mod "+Tab")) fwm-move-point-to-cursor)
     (cons (fwm-parse-key-combo (string-append mod "+Return")) (lambda (x) (exec terminal)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+Return")) (lambda (wm) (fwm-new-window-at wm (place-layout-slot wm))))
     (cons (fwm-parse-key-combo (string-append mod "+e")) (lambda (x) (exec "rofi -show run")))
     (cons (fwm-parse-key-combo (string-append mod "+q")) (lambda (x) (exec "xscreensaver-command -lock")))
     (cons (fwm-parse-key-combo (string-append mod "+x")) (lambda (x) (set-wallpaper)))
     (cons (fwm-parse-key-combo (string-append mod "+y")) (lambda (x) (set-wallpaper-killing-future)))
     (cons (fwm-parse-key-combo (string-append mod "+g")) (lambda (x)
							    (clear-wallpaper)
							    (set-wallpaper)
							    ))
     (cons (fwm-parse-key-combo (string-append mod "+0")) (lambda (wm) (switch-to-root wm 0)))
     (cons (fwm-parse-key-combo (string-append mod "+1")) (lambda (wm) (switch-to-root wm 1)))
     (cons (fwm-parse-key-combo (string-append mod "+2")) (lambda (wm) (switch-to-root wm 2)))
     (cons (fwm-parse-key-combo (string-append mod "+3")) (lambda (wm) (switch-to-root wm 3)))
     (cons (fwm-parse-key-combo (string-append mod "+4")) (lambda (wm) (switch-to-root wm 4)))
     (cons (fwm-parse-key-combo (string-append mod "+5")) (lambda (wm) (switch-to-root wm 5)))
     (cons (fwm-parse-key-combo (string-append mod "+6")) (lambda (wm) (switch-to-root wm 6)))
     (cons (fwm-parse-key-combo (string-append mod "+7")) (lambda (wm) (switch-to-root wm 7)))
     (cons (fwm-parse-key-combo (string-append mod "+8")) (lambda (wm) (switch-to-root wm 8)))
     (cons (fwm-parse-key-combo (string-append mod "+9")) (lambda (wm) (switch-to-root wm 9)))
     ;; (cons (fwm-parse-key-combo (string-append mod "+backslash")) (at-point fwm-toggle-map))
     (cons (fwm-parse-key-combo (string-append mod "+shift+x"))
	   (lambda (x)
	     (let ([wp (wall-back)])
	       (if wp
		   (do-set-wp wp)))))
     (cons (fwm-parse-key-combo (string-append mod "+Print")) (lambda (_) (copy-ss)))
     (cons (fwm-parse-key-combo (string-append mod "+z"))
	   (at-point unprotect-all request-kill-all
		     ))
     (cons (fwm-parse-key-combo (string-append mod "+shift+z"))
	   (at-point protect-all request-kill-all))
     (cons (fwm-parse-key-combo (string-append mod "+F1"))
	   (lambda (x) (println protected-points)))
     (cons (fwm-parse-key-combo (string-append mod "+F2"))
	   (lambda (wm) (let ([layout (fwm-get-layout wm)])
			  (println (pretty-print layout)))))
     (cons (fwm-parse-key-combo (string-append mod "+Up"))
	   (at-point increase-length))
     (cons (fwm-parse-key-combo (string-append mod "+Down"))
	   (at-point decrease-length))
     (cons (fwm-parse-key-combo (string-append mod "+equal"))
	   (at-point fwm-equalize-lengths))
     (cons (fwm-parse-key-combo (string-append mod "+minus"))
	   (lambda (wm) (fwm-show-root wm '(42))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+minus"))
	   (lambda (wm) (fwm-show-root wm '())))
     )
    )
  )

(define place-new-window-at-point
  (lambda (wm)
    (let* ([point (fwm-get-point wm)]
	   )
      (if (fwm-occupied? wm point)
	  (let* ([container (fwm-nearest-container wm point)]
		 [n_ctr_children (fwm-n-children wm container)])
	    (fwm-make-cursor-into container n_ctr_children) ; Insert at end of the container
	    )
					; The point is unoccupied, so let's insert there.
	  (cons 'Replace point)
	  )
      )
    )
  )
	   
(define place-new-window
  (lambda (wm)
    (let ([cursor (rust-option-to-scheme (fwm-get-cursor wm))])
      (if cursor (cons 'Move cursor)
             (place-new-window-at-point wm)))))

(define focus-if-window
  (lambda (wm point)
    (when (eq? (car point) 'Window)
      (fwm-set-focus wm (list (cdr point))))))

(define protected-points '())

(define roots (make-vector 10 #f))

(define (switch-to-root wm idx)
  (unless (vector-ref roots idx)
    (let ([root (fwm-alloc-root wm)])
      (vector-set! roots idx root))
    (println "roots now:" roots))
  (let ([root (vector-ref roots idx)])
    (println "setting point:" `(Container . ,root))
    (fwm-set-point wm `(Container . ,root))
    (fwm-show-root wm `(,root))))

(fwm-run-wm
 (list
  (cons 'bindings  bindings)
  (cons 'place-new-window place-new-window)
  (cons 'on-point-changed focus-if-window)
  (cons 'on-client-destroyed
	(lambda (wm point)
	  (println "on-client-destroyed:" point)
	  (if (not (member point protected-points))
		   (fwm-kill-item-at wm point))
	  ))
  (cons 'on-button1-pressed
	(lambda (wm point)
      (let ([point (rust-option-to-scheme point)])
          (println "on-button1-pressed:" point)
          (if point
              (fwm-set-point wm point)))))
  
 (cons 'after-start
       (lambda (wm)
	 (exec "xmobar")
         (exec "stalonetray --window-strut top")
         (switch-to-root wm 1)         
         ))
 ))


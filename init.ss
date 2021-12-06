(define at-point
  (lambda (f)
    (lambda (wm)
      (f wm (fwm-get-point wm)))))

(define terminal "~/.local/bin/sakura")
(define exec
  (lambda (cmd)
    (system (string-append cmd "&"))))

(system "vmware-user-suid-wrapper")
(system "xmodmap ~/.Xmodmap")
(exec "xscreensaver -no-splash")

(define random-wallpaper
  (lambda ()
    (use-modules (ice-9 ftw))
    (let* ([wp-dir "./wallpapers/"]
           [wps (scandir wp-dir (lambda (f) (or (string-suffix? ".jpg" f) (string-suffix? ".png" f))))]
           [idx (random (length wps))]
           [entry (list-ref wps idx)])
      (string-append wp-dir entry))))

(define set-wallpaper
  (lambda ()
    (system (string-append "feh --bg-max " (random-wallpaper)))))

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
           (display container)
           (display " ")
           (display index)
           (display "\n")
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
      (display item)
      (display "\n")
      (if item (fwm-set-cursor wm (list (make-split-cursor item dir)))))))
      
(define place-layout-slot
  (lambda (wm)
    (let ([cursor (rust-option-to-scheme (fwm-get-cursor wm))])
      (if cursor (cons 'Move cursor)
	  (let* ([point (fwm-get-point wm)]
		 [container (fwm-nearest-container wm point)]
		 [n_ctr_children (fwm-n-children wm container)])
	    (fwm-make-cursor-into container n_ctr_children))))))

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
     (cons (fwm-parse-key-combo (string-append mod "+p")) fwm-dump-layout)
     (cons (fwm-parse-key-combo (string-append mod "+v")) (lambda (wm) (set-split wm 'Down)))
     (cons (fwm-parse-key-combo (string-append mod "+m")) (lambda (wm) (set-split wm 'Right)))
     (cons (fwm-parse-key-combo (string-append mod "+Escape")) (lambda (wm) (fwm-set-cursor wm '())))
     (cons (fwm-parse-key-combo (string-append mod "+Tab")) fwm-move-point-to-cursor)
					; (cons (fwm-parse-key-combo (string-append mod "+m")) fwm-split-Right)
					; (cons (fwm-parse-key-combo (string-append mod "+v")) fwm-split-Down)
					;        (cons (fwm-parse-key-combo (string-append mod "+M"))
					;            (lambda (wm)
					;                (fwm-set-cursor wm (fwm-make-cursor (fwm-cursor-item (fwm-get-cursor wm)) '(Planar . Right)))
					;            )
					;        )
					;        (cons (fwm-parse-key-combo (string-append mod "+V")) 
					;            (lambda (wm)
					;                (fwm-set-cursor wm (fwm-make-cursor (fwm-cursor-item (fwm-get-cursor wm)) '(Planar . Down)))
					;            )
					;        )
					;        (cons (fwm-parse-key-combo (string-append mod "+g")) fwm-move)
					;        (cons (fwm-parse-key-combo (string-append mod "+G")) fwm-cursor-to-point)
     (cons (fwm-parse-key-combo (string-append mod "+Return")) (lambda (x) (exec terminal)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+Return")) (lambda (wm) (fwm-new-window-at wm (place-layout-slot wm))))
     (cons (fwm-parse-key-combo (string-append mod "+e")) (lambda (x) (exec "rofi -show run")))
     (cons (fwm-parse-key-combo (string-append mod "+q")) (lambda (x) (exec "xscreensaver-command -lock")))
     (cons (fwm-parse-key-combo (string-append mod "+x")) (lambda (x) (set-wallpaper)))
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
		    

(fwm-run-wm
 (list
  (cons 'bindings  bindings)
  (cons 'place-new-window place-new-window)
  (cons 'on-point-changed focus-if-window)
  )
 )

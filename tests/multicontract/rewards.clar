;; rewards contract — distributes rewards to users
(define-map claimed principal bool)
(define-data-var reward-amount uint u100)

(define-public (distribute (recipient principal))
    (begin
        (asserts! (is-eq tx-sender contract-caller) (err u1))
        (map-set claimed recipient true)
        (ok true)))

(define-public (set-reward (amount uint))
    (begin
        (var-set reward-amount amount)
        (ok true)))
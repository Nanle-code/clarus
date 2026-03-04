;; token contract — handles ft transfers
(define-map balances principal uint)

(define-public (transfer (amount uint) (sender principal) (recipient principal))
    (let ((sender-balance (default-to u0 (map-get? balances sender))))
        (asserts! (>= sender-balance amount) (err u1))
        (map-set balances sender (- sender-balance amount))
        (map-set balances recipient (+ sender-balance amount))
        (ok true)))

(define-read-only (get-balance (who principal))
    (ok (default-to u0 (map-get? balances who))))
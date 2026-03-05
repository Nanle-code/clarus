;; DEX Contract
;; Demonstrates trait-based dispatch vulnerability

(define-trait ft-trait
    (
        (transfer (uint principal principal) (response bool uint))
        (get-balance (principal) (response uint uint))
    )
)

(define-map reserves principal uint)
(define-data-var total-volume uint u0)

;; VULNERABLE: accepts any ft-trait implementor
;; a malicious contract implementing ft-trait could re-enter
(define-public (swap (token <ft-trait>) (amount uint) (recipient principal))
    (let ((reserve (default-to u0 (map-get? reserves recipient))))
        (contract-call? token transfer amount tx-sender recipient)
        (map-set reserves recipient (+ reserve amount))
        (var-set total-volume (+ (var-get total-volume) amount))
        (ok true)))

;; VULNERABLE: same pattern with different trait usage
(define-public (add-reserve (token <ft-trait>) (amount uint))
    (let ((current (default-to u0 (map-get? reserves tx-sender))))
        (contract-call? token transfer amount tx-sender (as-contract tx-sender))
        (map-set reserves tx-sender (+ current amount))
        (ok true)))

;; SAFE: read only — no state mutation around trait call
(define-read-only (get-reserve (who principal))
    (ok (default-to u0 (map-get? reserves who))))
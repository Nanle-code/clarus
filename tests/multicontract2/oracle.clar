;; Price Oracle Contract
;; Vulnerabilities: missing access control, unchecked returns

(define-map prices { token: principal } uint)
(define-map authorized-reporters principal bool)
(define-data-var last-update-block uint u0)
(define-data-var min-reporters uint u3)

;; VULNERABLE: anyone can update prices
(define-public (update-price (token principal) (price uint))
    (begin
        (map-set prices { token: token } price)
        (var-set last-update-block block-height)
        (ok true)))

;; VULNERABLE: anyone can add reporters
(define-public (add-reporter (reporter principal))
    (begin
        (map-set authorized-reporters reporter true)
        (ok true)))

;; VULNERABLE: anyone can remove reporters
(define-public (remove-reporter (reporter principal))
    (begin
        (map-delete authorized-reporters reporter)
        (ok true)))

;; VULNERABLE: unchecked return on notification
(define-public (update-and-notify (token principal) (price uint) (listener principal))
    (begin
        (asserts! (is-eq tx-sender contract-caller) (err u1))
        (map-set prices { token: token } price)
        (contract-call? .governance notify-price-update token price)
        (ok true)))

;; SAFE: read only price fetch
(define-read-only (get-price (token principal))
    (ok (default-to u0 (map-get? prices { token: token }))))
## Bestellliste

search = Bestellungen durchsuchen
    .placeholder = Kunden oder Bestellnummern suchen
status-filter = Status
    .all = Alle Status
sort = Sortieren nach

sort-newest = Neueste zuerst
sort-oldest = Älteste zuerst
sort-highest-total = Höchster Betrag
sort-lowest-total = Niedrigster Betrag

no-results = Keine Bestellungen gefunden
    .description = Keine Bestellung passt zu deiner Suche und dem Status.
clear-filters = Filter zurücksetzen

column-order = Bestellung
column-customer = Kunde
column-status = Status
column-items = Artikel
column-placed = Bestellt
column-total = Summe

# "Artikel" hat im Plural dieselbe Form.
item-count = { $count } Artikel

just-now = gerade eben
minutes-ago =
    { $count ->
        [one] vor { $count } Minute
       *[other] vor { $count } Minuten
    }
hours-ago =
    { $count ->
        [one] vor { $count } Stunde
       *[other] vor { $count } Stunden
    }
days-ago =
    { $count ->
        [one] vor { $count } Tag
       *[other] vor { $count } Tagen
    }

status-pending = Ausstehend
status-paid = Bezahlt
status-shipped = Versandt
status-delivered = Zugestellt
status-cancelled = Storniert
status-refunded = Erstattet

## Bestelldetails

# `$id` ist die Bestellnummer.
order-title = Bestellung #{ $id }
# `$date` ist der Zeitpunkt der Bestellung.
placed-on = Bestellt am { $date }

items = Artikel
unit-count = { $count } Stück
column-product = Produkt
column-quantity = Menge
column-price = Preis
column-subtotal = Zwischensumme
total = Summe

customer = Kunde
city = Stadt
customer-since = Kunde seit

# Schaltflächen, die die Bestellung in einen anderen Status versetzen.
action-pending = Als ausstehend markieren
action-paid = Als bezahlt markieren
action-shipped = Als versandt markieren
action-delivered = Als zugestellt markieren
action-cancelled = Bestellung stornieren
action-refunded = Bestellung erstatten

# Fragt nach einer Bestätigung für eine Statusänderung, die sich nicht
# rückgängig machen lässt. `$status` ist der neue Status, etwa "cancelled".
confirm-status =
    { $status ->
        [cancelled] Bestellung stornieren?
        [refunded] Bestellung erstatten?
       *[other] Status ändern?
    }
final-status = Bestellung #{ $id } kann danach nicht mehr in einen anderen Status wechseln.
keep-order = Bestellung behalten

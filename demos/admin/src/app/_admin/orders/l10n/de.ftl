## Bestellliste

search = Bestellungen durchsuchen
    .placeholder = Kunden oder Bestellnummern suchen
status_filter = Status
    .all = Alle Status
sort = Sortieren nach

sort_newest = Neueste zuerst
sort_oldest = Älteste zuerst
sort_highest_total = Höchster Betrag
sort_lowest_total = Niedrigster Betrag

no_results = Keine Bestellungen gefunden
    .description = Keine Bestellung passt zu deiner Suche und dem Status.
clear_filters = Filter zurücksetzen

column_order = Bestellung
column_customer = Kunde
column_status = Status
column_items = Artikel
column_placed = Bestellt
column_total = Summe

# "Artikel" hat im Plural dieselbe Form.
item_count = { $count } Artikel

just_now = gerade eben
minutes_ago =
    { $count ->
        [one] vor { $count } Minute
       *[other] vor { $count } Minuten
    }
hours_ago =
    { $count ->
        [one] vor { $count } Stunde
       *[other] vor { $count } Stunden
    }
days_ago =
    { $count ->
        [one] vor { $count } Tag
       *[other] vor { $count } Tagen
    }

status_pending = Ausstehend
status_paid = Bezahlt
status_shipped = Versandt
status_delivered = Zugestellt
status_cancelled = Storniert
status_refunded = Erstattet

## Bestelldetails

# `$id` ist die Bestellnummer.
order_title = Bestellung #{ $id }
# `$date` ist der Zeitpunkt der Bestellung.
placed_on = Bestellt am { $date }

items = Artikel
unit_count = { $count } Stück
column_product = Produkt
column_quantity = Menge
column_price = Preis
column_subtotal = Zwischensumme
total = Summe

customer = Kunde
city = Stadt
customer_since = Kunde seit

# Schaltflächen, die die Bestellung in einen anderen Status versetzen.
action_pending = Als ausstehend markieren
action_paid = Als bezahlt markieren
action_shipped = Als versandt markieren
action_delivered = Als zugestellt markieren
action_cancelled = Bestellung stornieren
action_refunded = Bestellung erstatten

# Fragt nach einer Bestätigung für eine Statusänderung, die sich nicht
# rückgängig machen lässt. `$status` ist der neue Status, etwa "cancelled".
confirm_status =
    { $status ->
        [cancelled] Bestellung stornieren?
        [refunded] Bestellung erstatten?
       *[other] Status ändern?
    }
final_status = Bestellung #{ $id } kann danach nicht mehr in einen anderen Status wechseln.
keep_order = Bestellung behalten

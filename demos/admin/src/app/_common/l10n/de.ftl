# Ein Betrag in US-Dollar, etwa 1234.5. Das geschützte Leerzeichen hält Betrag
# und Währung in einer Zeile.
money = { NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2) }{ "\u00A0" }$
# Ein Datum mit Uhrzeit in UTC.
timestamp = { DATETIME($at, dateStyle: "medium", timeStyle: "short") } UTC

role_admin = Admin
role_staff = Mitarbeiter
role_viewer = Betrachter

# Ein Betrag in US-Dollar, etwa 1234.5. Das geschützte Leerzeichen hält Betrag
# und Währung in einer Zeile.
money = { NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2) }{ " " }$

role-admin = Admin
role-staff = Mitarbeiter
role-viewer = Betrachter

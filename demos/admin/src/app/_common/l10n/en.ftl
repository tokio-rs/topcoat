# An amount in US dollars, such as 1234.5.
money = ${ NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2) }
# A date and time of day in UTC.
timestamp = { DATETIME($at, dateStyle: "medium", timeStyle: "short") } UTC

role_admin = Admin
role_staff = Staff
role_viewer = Viewer

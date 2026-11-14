# Storefront date formats, used with `to_fs`.
Date::DATE_FORMATS[:storefront] = "%b %-d, %Y"            # Mar 4, 2026
Time::DATE_FORMATS[:storefront] = "%b %-d, %Y %H:%M"      # Mar 4, 2026 14:05
Time::DATE_FORMATS[:iso8601_minutes] = "%Y-%m-%dT%H:%MZ"  # 2026-03-04T14:05Z, for UTC times

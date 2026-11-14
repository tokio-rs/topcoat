# A product category from `categories.json`.
class Category < Data.define(:slug, :name, :description)
  def self.from_json(row)
    new(slug: row["slug"], name: row["name"], description: row["description"])
  end
end

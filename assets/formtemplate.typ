// We need to set num_qs, num_idqs, num_answers and num_versions

#set page("a4", margin: (x: 1.5cm, top: 0.5cm, bottom: 1cm))

// Helper function to get letter for index (0 -> A, 1 -> B, etc)
#let index_to_letter(i) = {
  let letters = ("A", "B", "C", "D", "E", "F", "G", "H", "I", "J")
  if i < letters.len() {
    letters.at(i)
  } else {
    "?"
  }
}

#let bubble(id: none, body) = {
  [#metadata((type: "bubble", id: id)) #circle(
    inset: 1pt,
    outset: 2pt,
    fill: none,
    stroke: black,
    [#text(size: 5pt)[#body]]
  )]
}

#let version_bubble(version) = {
  bubble(id: "version-" + version)[#version]
}

#let mcq_bubble(q_num, option) = {
  bubble(id: "mcq-" + str(q_num) + "-" + option)[#option]
}

#let id_bubble(row, digit) = {
  bubble(id: "id-" + str(row) + "-" + str(digit))[#digit]
}

#let annulus = [#circle(
  fill: black,
  inset: 12%,
  [#circle(radius: 15pt, fill: white)]
)]

// Generate table rows for MCQ section
#let mcq_rows = for i in range(1, num_qs + 1) {
  let row_items = ([#i.],)
  for j in range(num_answers) {
    row_items.push(mcq_bubble(i, index_to_letter(j)))
  }
  row_items
}

// Generate version bubbles
#let version_items = ([Version:],)
#for i in range(num_versions) {
  version_items.push(version_bubble(index_to_letter(i)))
}

// Generate ID rows
#let id_rows = for i in range(1, num_idqs + 1) {
  let row_items = ([],)
  for j in range(10) {
    row_items.push(id_bubble(i, j))
  }
  row_items
}

#grid(columns: (1.5fr, 2fr),
align: (left, left),
inset: 5pt,
[#annulus #v(1cm)], grid.cell(align: right, [#annulus]),
grid.cell(
rowspan: 5,
[#table(
  columns: num_answers + 1,
  rows: (auto, auto, auto, auto, auto),
  align: right,
  stroke: none,
  inset: 4pt,
  ..mcq_rows
)]), 
[#table(
  columns: 2,
  align: horizon,
  inset: 10pt,
  stroke: none,
  [Name:], table.cell(align: bottom, line(length: 3cm)),
  [Section:], table.cell(align: bottom, line(length: 3cm)) 
  )
],
[#table(
  columns: num_versions + 1,
  align: horizon,
  stroke: none,
  inset: 10pt,
  ..version_items
)], 
[#table(
  columns: 11,
  align: horizon,
  stroke: (x, y) => if x==0 { (right: 1pt, left: 1pt, bottom: 1pt, top:1pt) } else if x==10 { (right: 1pt, bottom: 1pt, top: 1pt) } else { (top: 1pt, bottom: 1pt) },
  [ID\#], [0], [1], [2], [3], [4], [5],[6],[7],[8],[9],
  ..id_rows
)], 
[#v(1cm)If you make a mistake, do *NOT* mark it with X or use an eraser. Instead, use blanco or ask for a new bubble sheet.],
grid.cell(align: right, [#v(1fr) #annulus])
)

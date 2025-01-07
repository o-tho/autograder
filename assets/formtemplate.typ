//#let num_qs = 20
//#let num_idqs = 9
//#let num_answers = 5
//#let num_versions = 4

#set page("a4", margin: (x: 1.5cm, top: 0.5cm, bottom: 1cm))

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

#let id_bubble(col, digit) = {
  bubble(id: "id-" + str(col) + "-" + str(digit))[#digit]
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
// Generate ID table content as a single flattened sequence
#let id_rows = {
  // First row with ID label and empty cells for student input
  let header = ([ID:],)
  for i in range(num_idqs) {
    header.push([ ])
  }
  
  // Generate rows for digits 0-9
  let all_rows = (header,)
  for digit in range(10) {
    let row = ([ ],)
    for col in range(num_idqs) {
      row.push(id_bubble(col, digit))
    }
    all_rows.push(row)
  }
  
  // Flatten the rows into a single sequence
  all_rows.flatten()
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
  [Name:], table.cell(align: bottom, line(length: 4cm)),
  [Section:], table.cell(align: bottom, line(length: 4cm)) 
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
  columns: num_idqs + 1,
  align: (x,y) => if x==0{(right)} else {(horizon)},
  stroke: (x, y) => if x == 0 {
    none  // No borders for ID column
  } else if y == 0 {
    if x == num_idqs {
      (left: 1pt, top: 1pt, bottom: 1pt, right: 1pt)  // Last column of freetext
    } else {
      (left: 1pt, top: 1pt, bottom: 1pt)  // Other freetext columns
    }
  } else {
    if x == num_idqs {
      (left: 1pt, bottom: if y == 10 { 1pt } else { none }, right: 1pt)  // Last digit column
    } else {
      (left: 1pt, bottom: if y == 10 { 1pt } else { none })  // Other digit columns
    }
  },
  ..id_rows
)], 
[#v(1cm)If you make a mistake, do *NOT* mark it with X or use an eraser. Instead, use blanco or ask for a new bubble sheet.],
grid.cell(align: right, [#v(1fr) #annulus])
)

import React from 'react';
import { Link } from 'react-router-dom';

import { EXAMPLES, ExampleDefinition } from '../components/registry';

const groupExamples = () => {
  const grouped: { group: string; items: ExampleDefinition[] }[] = [];
  const groupIndex = new Map<string, number>();

  EXAMPLES.forEach((example: ExampleDefinition) => {
    if (!groupIndex.has(example.group)) {
      groupIndex.set(example.group, grouped.length);
      grouped.push({ group: example.group, items: [] });
    }

    const index = groupIndex.get(example.group)!;
    grouped[index].items.push(example);
  });

  return grouped;
};

const groupedExamples = groupExamples();

const Home: React.FC = () => (
  <>
    <h1>finstack-wasm TypeScript Examples</h1>
    <p className="intro">
      Explore focused walkthroughs of the wasm bindings. Pick an area below to open a dedicated page
      for the example, mirroring the Python tutorials.
    </p>

    {groupedExamples.map(({ group, items }) => (
      <section key={group} className="example-group">
        <h2>{group}</h2>
        <div className="example-grid">
          {items.map((example) => (
            <Link key={example.slug} to={`/examples/${example.slug}`} className="example-card">
              <h3>{example.title}</h3>
              <p>{example.description}</p>
              <span className="example-card__cta">View example →</span>
            </Link>
          ))}
        </div>
      </section>
    ))}
  </>
);

export default Home;

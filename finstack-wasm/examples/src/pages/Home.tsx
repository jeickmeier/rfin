import React from 'react';
import { Link } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { EXAMPLES, ExampleDefinition } from '../components/registry';

const groupExamples = () => {
  const grouped: { group: string; items: ExampleDefinition[] }[] = [];
  const groupIndex = new Map<string, number>();

  EXAMPLES.forEach((example: ExampleDefinition) => {
    if (!groupIndex.has(example.group)) {
      groupIndex.set(example.group, grouped.length);
      grouped.push({ group: example.group, items: [] });
    }

    const index = groupIndex.get(example.group);
    if (index !== undefined) {
      grouped[index].items.push(example);
    }
  });

  return grouped;
};

const groupedExamples = groupExamples();

const Home: React.FC = () => (
  <div className="space-y-12">
    <header className="text-center">
      <Badge variant="secondary" className="mb-4">
        WebAssembly
      </Badge>
      <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">finstack-wasm Examples</h1>
      <p className="mx-auto mt-4 max-w-2xl text-lg text-muted-foreground">
        Explore focused walkthroughs of the wasm bindings. Pick an area below to open a dedicated
        page for the example, mirroring the Python tutorials.
      </p>
    </header>

    {groupedExamples.map(({ group, items }) => (
      <section key={group} className="space-y-6">
        <h2 className="text-2xl font-semibold tracking-tight text-primary">{group}</h2>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {items.map((example) => (
            <Link key={example.slug} to={`/examples/${example.slug}`} className="group">
              <Card className="h-full transition-all duration-200 hover:border-primary hover:shadow-lg hover:-translate-y-1">
                <CardHeader>
                  <CardTitle className="text-lg group-hover:text-primary transition-colors">
                    {example.title}
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <CardDescription className="line-clamp-3">{example.description}</CardDescription>
                  <span className="inline-flex items-center text-sm font-medium text-primary">
                    View example
                    <svg
                      className="ml-1 h-4 w-4 transition-transform group-hover:translate-x-1"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 5l7 7-7 7"
                      />
                    </svg>
                  </span>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      </section>
    ))}
  </div>
);

export default Home;

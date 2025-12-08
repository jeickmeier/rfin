import React from 'react';
import { Link, useParams } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { getExampleBySlug } from '../components/registry';
import NotFound from './NotFound';

const ExamplePage: React.FC = () => {
  const { slug } = useParams<{ slug: string }>();
  const example = slug ? getExampleBySlug(slug) : undefined;

  if (!example) {
    return <NotFound />;
  }

  const { Component } = example;

  return (
    <div className="space-y-6">
      <nav>
        <Button variant="ghost" asChild className="-ml-4">
          <Link to="/">
            <svg className="mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 19l-7-7 7-7"
              />
            </svg>
            All examples
          </Link>
        </Button>
      </nav>

      <header className="space-y-2">
        <h1 className="text-3xl font-bold tracking-tight">{example.title}</h1>
        <p className="text-lg text-muted-foreground">{example.description}</p>
      </header>

      <Separator />

      <Component />
    </div>
  );
};

export default ExamplePage;

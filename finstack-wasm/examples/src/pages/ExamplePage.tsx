import React from 'react';
import { Link, useParams } from 'react-router-dom';

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
    <div className="example-page">
      <nav className="breadcrumb">
        <Link to="/">← All examples</Link>
      </nav>

      <header>
        <h1>{example.title}</h1>
        <p className="page-lead">{example.description}</p>
      </header>

      <Component />
    </div>
  );
};

export default ExamplePage;

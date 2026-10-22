export function Actions() {
  return (
    <>
      <button className="focus-visible:outline-none focus-visible:ring-2">Save</button>
      <button className="focus:outline-none dark:focus:border-white">Save</button>
      <div className="md:focus-visible:outline-2" />
      <div className="[&:focus-visible]:outline-2" />
      <div className="data-[focus]:outline-2" />
      <div className="outline-none" />
      <div className="focus:not(:focus-visible)" />
    </>
  );
}
